//! [`AuthBackend`] built on the `atrium-api` crates (app-password sessions).
//!
//! Nothing outside this file should name an atrium type, so this backend can be
//! replaced by a custom client (or an OAuth one) without touching callers.

use super::xrpc::Xrpc;
use super::{AuthBackend, AuthError, Credentials, Session};
use atrium_api::agent::AuthorizationProvider;
use atrium_api::agent::atp_agent::store::{AtpSessionStore, MemorySessionStore};
use atrium_api::agent::atp_agent::{AtpAgent, AtpSession};
use atrium_api::com::atproto::server::{create_session, refresh_session};
use atrium_api::did_doc::DidDocument;
use atrium_api::types::TryFromUnknown;
use atrium_api::xrpc::error::{Error as XrpcError, XrpcErrorKind};
use atrium_api::xrpc::types::AuthorizationToken;
use atrium_common::store::Store;
use std::fmt::Debug;

#[derive(Debug, Default, Clone)]
pub struct AtriumBackend;

impl AtriumBackend {
    pub fn new() -> Self {
        Self
    }

    /// An agent pointed at the session's PDS and pre-loaded with its tokens.
    async fn agent_for<S>(
        &self,
        session: &Session,
        store: S,
    ) -> Result<AtpAgent<S, Xrpc>, AuthError>
    where
        S: AtpSessionStore + Send + Sync,
        S::Error: std::error::Error + Send + Sync + 'static,
    {
        store
            .set((), to_atp(session)?)
            .await
            .map_err(|e| AuthError::Transport(e.to_string()))?;
        Ok(AtpAgent::new(Xrpc::new(&session.pds_url), store))
    }
}

impl AuthBackend for AtriumBackend {
    async fn login(&self, creds: &Credentials) -> Result<Session, AuthError> {
        let agent = AtpAgent::new(Xrpc::new(&creds.service), MemorySessionStore::default());
        // Called directly rather than via `AtpAgent::login` so we can pass a 2FA code.
        let out = agent
            .api
            .com
            .atproto
            .server
            .create_session(
                create_session::InputData {
                    allow_takendown: None,
                    auth_factor_token: creds.auth_factor_token.clone(),
                    identifier: creds.identifier.clone(),
                    password: creds.password.clone(),
                }
                .into(),
            )
            .await
            .map_err(map_err)?;
        let pds_url = pds_from(out.did_doc.as_ref()).unwrap_or_else(|| creds.service.clone());
        Ok(Session {
            did: out.did.to_string(),
            handle: out.handle.to_string(),
            pds_url,
            access_jwt: out.data.access_jwt,
            refresh_jwt: out.data.refresh_jwt,
        })
    }

    async fn refresh(&self, session: &Session) -> Result<Session, AuthError> {
        let agent = self
            .agent_for(session, MemorySessionStore::default())
            .await?;
        // atrium sends the refresh token for this call.
        let out: refresh_session::Output = agent
            .api
            .com
            .atproto
            .server
            .refresh_session()
            .await
            .map_err(map_err)?;
        Ok(Session {
            did: out.did.to_string(),
            handle: out.handle.to_string(),
            pds_url: pds_from(out.did_doc.as_ref()).unwrap_or_else(|| session.pds_url.clone()),
            access_jwt: out.data.access_jwt,
            refresh_jwt: out.data.refresh_jwt,
        })
    }

    async fn validate(&self, session: &Session) -> Result<Session, AuthError> {
        let agent = self
            .agent_for(session, MemorySessionStore::default())
            .await?;
        // On ExpiredToken atrium refreshes and retries once, updating its store.
        let out = agent
            .api
            .com
            .atproto
            .server
            .get_session()
            .await
            .map_err(map_err)?;
        let current = agent.get_session().await.ok_or(AuthError::SessionExpired)?;
        Ok(Session {
            did: out.did.to_string(),
            handle: out.handle.to_string(),
            pds_url: pds_from(out.did_doc.as_ref()).unwrap_or_else(|| session.pds_url.clone()),
            access_jwt: current.data.access_jwt,
            refresh_jwt: current.data.refresh_jwt,
        })
    }

    async fn logout(&self, session: &Session) -> Result<(), AuthError> {
        // deleteSession must be authorized with the refresh token, but atrium only
        // sends it for refreshSession, so use a store that always hands it out.
        let agent = self
            .agent_for(session, RefreshTokenStore::default())
            .await?;
        agent
            .api
            .com
            .atproto
            .server
            .delete_session()
            .await
            .map_err(map_err)
    }
}

/// Session store whose authorization is always the refresh token.
#[derive(Default)]
struct RefreshTokenStore(MemorySessionStore);

impl Store<(), AtpSession> for RefreshTokenStore {
    type Error = <MemorySessionStore as Store<(), AtpSession>>::Error;
    async fn get(&self, key: &()) -> Result<Option<AtpSession>, Self::Error> {
        self.0.get(key).await
    }
    async fn set(&self, key: (), value: AtpSession) -> Result<(), Self::Error> {
        self.0.set(key, value).await
    }
    async fn del(&self, key: &()) -> Result<(), Self::Error> {
        self.0.del(key).await
    }
    async fn clear(&self) -> Result<(), Self::Error> {
        self.0.clear().await
    }
}

impl AuthorizationProvider for RefreshTokenStore {
    async fn authorization_token(&self, _is_refresh: bool) -> Option<AuthorizationToken> {
        self.0.authorization_token(true).await
    }
}

impl AtpSessionStore for RefreshTokenStore {}

pub(crate) fn to_atp(s: &Session) -> Result<AtpSession, AuthError> {
    let bad = |what: &str| AuthError::Transport(format!("stored session has an invalid {what}"));
    Ok(create_session::OutputData {
        access_jwt: s.access_jwt.clone(),
        active: None,
        did: s.did.parse().map_err(|_| bad("DID"))?,
        did_doc: None,
        email: None,
        email_auth_factor: None,
        email_confirmed: None,
        handle: s.handle.parse().map_err(|_| bad("handle"))?,
        refresh_jwt: s.refresh_jwt.clone(),
        status: None,
    }
    .into())
}

fn pds_from(did_doc: Option<&atrium_api::types::Unknown>) -> Option<String> {
    DidDocument::try_from_unknown(did_doc?.clone())
        .ok()?
        .get_pds_endpoint()
}

pub(crate) fn map_err<E: Debug + std::fmt::Display>(err: XrpcError<E>) -> AuthError {
    let XrpcError::XrpcResponse(resp) = err else {
        return AuthError::Transport(err.to_string());
    };
    let name = match &resp.error {
        Some(XrpcErrorKind::Undefined(body)) => body.error.clone().unwrap_or_default(),
        // Lexicon-defined errors, e.g. `AuthFactorTokenRequired(Some("..."))`.
        Some(XrpcErrorKind::Custom(e)) => format!("{e:?}"),
        None => String::new(),
    };
    let name = name.split('(').next().unwrap_or_default();
    match name {
        "AuthFactorTokenRequired" => AuthError::AuthFactorRequired,
        "AccountTakedown" | "AccountDeactivated" => AuthError::AccountUnavailable,
        "ExpiredToken" | "InvalidToken" => AuthError::SessionExpired,
        // createSession answers a wrong password with 401 AuthenticationRequired; on any
        // other call a 401 means our tokens are no good.
        "AuthenticationRequired" if resp.status.as_u16() == 401 => AuthError::InvalidCredentials,
        _ if resp.status.as_u16() == 401 => AuthError::SessionExpired,
        _ => AuthError::Transport(format!("{} {name}", resp.status)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atrium_api::xrpc::error::{ErrorResponseBody, XrpcError as Resp};
    use atrium_api::xrpc::http::StatusCode;

    fn resp<E: Debug>(status: u16, kind: Option<XrpcErrorKind<E>>) -> XrpcError<E> {
        XrpcError::XrpcResponse(Resp {
            status: StatusCode::from_u16(status).unwrap(),
            error: kind,
        })
    }

    #[expect(clippy::unnecessary_wraps, reason = "matches what resp() takes")]
    fn undefined(name: &str) -> Option<XrpcErrorKind<create_session::Error>> {
        Some(XrpcErrorKind::Undefined(ErrorResponseBody {
            error: Some(name.into()),
            message: None,
        }))
    }

    #[test]
    fn maps_server_errors() {
        assert!(matches!(
            map_err(resp(
                401,
                Some(XrpcErrorKind::Custom(
                    create_session::Error::AuthFactorTokenRequired(Some("check email".into()))
                ))
            )),
            AuthError::AuthFactorRequired
        ));
        assert!(matches!(
            map_err(resp(400, undefined("ExpiredToken"))),
            AuthError::SessionExpired
        ));
        assert!(matches!(
            map_err(resp(401, undefined("AuthenticationRequired"))),
            AuthError::InvalidCredentials
        ));
        assert!(matches!(
            map_err(resp(502, undefined("UpstreamFailure"))),
            AuthError::Transport(_)
        ));
    }

    /// Hits bsky.social, so it is opt-in: `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore = "hits bsky.social"]
    async fn live_wrong_password_is_invalid_credentials() {
        let creds = Credentials {
            identifier: "bsky.app".into(),
            password: "definitely-not-the-app-password".into(),
            auth_factor_token: None,
            service: crate::auth::DEFAULT_SERVICE.into(),
        };
        let err = AtriumBackend::new().login(&creds).await.unwrap_err();
        assert!(matches!(err, AuthError::InvalidCredentials), "got {err:?}");
    }

    #[test]
    fn session_converts_to_atrium() {
        let s = crate::auth::test_util::session("did:plc:abc", std::time::SystemTime::now());
        let atp = to_atp(&s).unwrap();
        assert_eq!(atp.did.as_str(), "did:plc:abc");
        assert_eq!(atp.refresh_jwt, s.refresh_jwt);

        let mut bad = s.clone();
        bad.did = "not a did".into();
        assert!(to_atp(&bad).is_err());
    }
}
