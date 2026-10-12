//! Signing in through the browser with atproto OAuth.
//!
//! BiFrost is a "localhost" client: it needs no hosted metadata page, and the
//! authorization server sends the browser back to a one-shot listener on this
//! machine. Sessions (a DPoP key and tokens) are kept in the OS credential store and
//! refresh themselves.

use super::AuthError;
use super::handle::{self, HandleResolver};
use super::sessions::Sessions;
use super::xrpc::Xrpc;
use crate::storage::secrets::SecretStore;
use atrium_api::agent::bluesky::BSKY_LABELER_DID;
use atrium_api::agent::{Agent, Configure, SessionManager};
use atrium_api::types::string::Did;
use atrium_common::store::Store;
use atrium_identity::did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL};
use atrium_oauth::store::state::MemoryStateStore;
use atrium_oauth::{
    AtprotoLocalhostClientMetadata, AuthorizeOptions, CallbackParams, KnownScope, OAuthClient,
    OAuthClientConfig, OAuthResolverConfig, OAuthSession, Scope,
};
use atrium_xrpc::XrpcClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// The browser comes back here. Fixed, because it is part of the client's identity:
/// sessions can only be refreshed by the client that started them.
const REDIRECT_PORT: u16 = 45823;
/// How long to wait for the person to finish in the browser.
const SIGN_IN_TIMEOUT: Duration = Duration::from_mins(10);
/// How long one connection to the redirect listener may take to send its request.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

type Http = Xrpc;
type DidResolver = CommonDidResolver<Http>;
pub type OAuthAgent = Agent<OAuthSession<Http, DidResolver, HandleResolver, Sessions>>;

/// Who signed in.
#[derive(Debug, Clone)]
pub struct SignedIn {
    pub did: String,
    pub handle: String,
    pub pds_url: String,
}

pub struct OAuth {
    client: OAuthClient<MemoryStateStore, Sessions, DidResolver, HandleResolver, Http>,
    /// The same store the client keeps pending sign-ins in, to tell the real callback
    /// from a stray request before handing it over.
    states: MemoryStateStore,
    /// The same store the client keeps sessions in, to sign accounts out of it.
    sessions: Sessions,
}

impl OAuth {
    #[expect(
        clippy::default_trait_access,
        reason = "atrium-oauth does not export the resolver config types"
    )]
    pub fn new(secrets: Arc<dyn SecretStore>) -> Result<Self, AuthError> {
        let http = Arc::new(Xrpc::new("https://bsky.social"));
        let states = MemoryStateStore::default();
        let sessions = Sessions::new(secrets);
        let client = OAuthClient::new(OAuthClientConfig {
            client_metadata: AtprotoLocalhostClientMetadata {
                redirect_uris: Some(vec![redirect_uri()]),
                scopes: Some(scopes()),
            },
            keys: None,
            resolver: OAuthResolverConfig {
                did_resolver: CommonDidResolver::new(CommonDidResolverConfig {
                    plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
                    http_client: http.clone(),
                }),
                handle_resolver: handle::resolver(&http),
                authorization_server_metadata: Default::default(),
                protected_resource_metadata: Default::default(),
            },
            state_store: states.clone(),
            session_store: sessions.clone(),
            http_client: Xrpc::new("https://bsky.social"),
        })
        .map_err(|e| AuthError::Transport(e.to_string()))?;
        Ok(Self {
            client,
            states,
            sessions,
        })
    }

    /// Opens the sign-in page for the account `typed` names in the browser and waits
    /// for it to send the browser back.
    pub async fn sign_in(&self, typed: &str) -> Result<SignedIn, AuthError> {
        let input = handle::sign_in_input(typed)?;
        self.sessions.signing_in();
        // Listen first, so the browser never comes back to nothing.
        let listener = TcpListener::bind(("127.0.0.1", REDIRECT_PORT))
            .await
            .map_err(|e| {
                AuthError::Transport(format!(
                    "could not listen on port {REDIRECT_PORT} for the browser to come back ({e}). \
                     Is another BiFrost signing in?"
                ))
            })?;
        let url = self
            .client
            .authorize(
                &input,
                AuthorizeOptions {
                    scopes: scopes(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| handle::sign_in_error(&input, e))?;
        open::that_detached(sign_in_page(&url)?)
            .map_err(|e| AuthError::Transport(format!("could not open the browser: {e}")))?;

        let session = tokio::time::timeout(SIGN_IN_TIMEOUT, self.wait_for_browser(listener))
            .await
            .map_err(|_| AuthError::Transport("signing in took too long; try again".into()))??;

        let did = session.did().await.ok_or(AuthError::SessionExpired)?;
        let pds_url = session.base_uri();
        let agent = Agent::new(session);
        let me = agent
            .api
            .com
            .atproto
            .server
            .get_session()
            .await
            .map_err(super::atrium::map_err)?;
        Ok(SignedIn {
            did: did.to_string(),
            handle: me.handle.to_string(),
            pds_url,
        })
    }

    /// Serves the redirect: answers stray requests (like the favicon) and returns
    /// once the real callback has been exchanged for tokens.
    async fn wait_for_browser(
        &self,
        listener: TcpListener,
    ) -> Result<OAuthSession<Http, DidResolver, HandleResolver, Sessions>, AuthError> {
        loop {
            let (mut stream, _) = listener
                .accept()
                .await
                .map_err(|e| AuthError::Transport(e.to_string()))?;
            // A connection that sends nothing (a browser preconnect, say) must not hold
            // up the real callback.
            let Ok(buf) = tokio::time::timeout(READ_TIMEOUT, read_request(&mut stream)).await
            else {
                continue;
            };
            let request = String::from_utf8_lossy(&buf);
            let target = request.split_whitespace().nth(1).unwrap_or("");
            let Some(query) = target.strip_prefix("/callback?") else {
                // Answers here are courtesy to the browser; if it has gone, the
                // wait for the real callback carries on.
                let _ = respond(&mut stream, "404 Not Found", "Not found.").await;
                continue;
            };
            let params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();
            let get = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
            // Only a callback for the sign-in we started counts. Anything else, such
            // as a web page poking this port, is answered and ignored.
            let pending = match get("state") {
                Some(state) => self.states.get(&state).await.ok().flatten().is_some(),
                None => false,
            };
            if !pending {
                let _ = respond(
                    &mut stream,
                    "400 Bad Request",
                    "This sign-in link doesn't match the one BiFrost is waiting for.",
                )
                .await;
                continue;
            }
            if let Some(error) = get("error") {
                let why = get("error_description").unwrap_or(error);
                let _ = respond(
                    &mut stream,
                    "200 OK",
                    "Signing in was cancelled. You can close this tab.",
                )
                .await;
                return Err(AuthError::Transport(format!(
                    "signing in was cancelled: {why}"
                )));
            }
            let Some(code) = get("code") else {
                let _ = respond(&mut stream, "400 Bad Request", "Missing code.").await;
                continue;
            };
            let result = self
                .client
                .callback(CallbackParams {
                    code,
                    state: get("state"),
                    iss: get("iss"),
                })
                .await;
            return match result {
                Ok((session, _)) => {
                    let _ = respond(
                        &mut stream,
                        "200 OK",
                        "You're signed in to BiFrost. You can close this tab.",
                    )
                    .await;
                    Ok(session)
                }
                Err(e) => {
                    let _ = respond(
                        &mut stream,
                        "200 OK",
                        "Signing in failed. Go back to BiFrost to try again.",
                    )
                    .await;
                    Err(AuthError::Transport(format!("signing in failed: {e}")))
                }
            };
        }
    }

    /// An agent for a signed-in account, refreshing its tokens if needed.
    pub async fn agent(&self, did: &str) -> Result<OAuthAgent, AuthError> {
        let did = Did::new(did.to_string()).map_err(|e| AuthError::Transport(e.to_string()))?;
        let session = self
            .client
            .restore(&did)
            .await
            .map_err(|_| AuthError::SessionExpired)?;
        let agent = Agent::new(session);
        // As for app-password sessions: Bluesky's own moderation labels.
        if let Ok(labeler) = BSKY_LABELER_DID.parse() {
            agent.configure_labelers_header(Some(vec![(labeler, true)]));
        }
        Ok(agent)
    }

    /// Revokes the account's tokens with its server. Best effort: the local copy is
    /// removed by the account manager either way.
    pub async fn revoke(&self, did: &str) {
        if let Ok(did) = Did::new(did.to_string()) {
            let _ = self.client.revoke(&did).await;
        }
        // Whatever the server said, a refresh still under way can't keep the tokens.
        let _ = self.sessions.sign_out(did);
    }
}

/// The sign-in page, only if it is an https address. It comes from the account's
/// server, and anything else (a file, a program) must not be handed to the system.
fn sign_in_page(url: &str) -> Result<&str, AuthError> {
    url::Url::parse(url)
        .is_ok_and(|u| u.scheme() == "https" && u.host_str().is_some())
        .then_some(url)
        .ok_or_else(|| AuthError::Transport("the sign-in page is not a secure web address".into()))
}

fn redirect_uri() -> String {
    format!("http://127.0.0.1:{REDIRECT_PORT}/callback")
}

fn scopes() -> Vec<Scope> {
    vec![
        Scope::Known(KnownScope::Atproto),
        Scope::Known(KnownScope::TransitionGeneric),
        Scope::Known(KnownScope::TransitionChatBsky),
    ]
}

/// Reads a request's head: up to 8 KB, or until the blank line after the headers.
async fn read_request(stream: &mut tokio::net::TcpStream) -> Vec<u8> {
    let mut buf = vec![0u8; 8192];
    let mut len = 0;
    while len < buf.len() {
        let n = stream.read(&mut buf[len..]).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        len += n;
        if buf[..len].windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    buf.truncate(len);
    buf
}

async fn respond(
    stream: &mut tokio::net::TcpStream,
    status: &str,
    message: &str,
) -> std::io::Result<()> {
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>BiFrost</title>\
         <body style=\"font:16px system-ui;background:#1b1b1e;color:#e8e8ea;\
         display:grid;place-items:center;height:90vh\"><p>{message}</p></body>"
    );
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(reply.as_bytes()).await?;
    stream.shutdown().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::secrets::MemoryStore;

    #[test]
    fn redirect_is_a_valid_localhost_client() {
        let store: Arc<dyn SecretStore> = Arc::new(MemoryStore::default());
        let oauth = OAuth::new(store).expect("client builds");
        assert!(
            oauth
                .client
                .client_metadata
                .client_id
                .starts_with("http://localhost?")
        );
        assert_eq!(
            oauth.client.client_metadata.redirect_uris,
            vec![redirect_uri()]
        );
    }

    #[test]
    fn only_https_sign_in_pages_open() {
        let page = "https://bsky.social/oauth/authorize?request_uri=x";
        assert_eq!(sign_in_page(page).ok(), Some(page));
        for bad in [
            "http://bsky.social/a",
            "file:///C:/Windows/calc.exe",
            "calc",
            "",
        ] {
            assert!(sign_in_page(bad).is_err(), "{bad}");
        }
    }
}

#[cfg(test)]
mod live {
    use super::*;
    use crate::storage::secrets::MemoryStore;

    #[tokio::test]
    #[ignore = "talks to bsky.social"]
    async fn authorize_reaches_bluesky() {
        let oauth = OAuth::new(Arc::new(MemoryStore::default())).unwrap();
        for input in ["bsky.app", "chama.pro", "https://bsky.social"] {
            let url = oauth
                .client
                .authorize(
                    input,
                    AuthorizeOptions {
                        scopes: scopes(),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
            println!("{input} -> {url}");
            assert!(url.contains("request_uri="));
        }
    }
}
