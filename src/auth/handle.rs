//! Finding the account behind what someone typed to sign in through the browser.
//!
//! A handle is checked the way the AT Protocol says: a DNS record, then a file on the
//! handle's own web site. When neither answers (a DNS filter, a blocked DNS-over-HTTPS
//! service, a site that is down), Bluesky's public API is asked instead, as Bluesky's
//! own apps do. Either way the account's DID document must name the handle back
//! before sign-in goes on.

use super::AuthError;
use super::xrpc::Xrpc;
use atrium_api::types::string::{Did, Handle};
use atrium_common::resolver::Resolver;
use atrium_identity::Error;
use atrium_identity::handle::{
    AppViewHandleResolver, AppViewHandleResolverConfig, AtprotoHandleResolver,
    AtprotoHandleResolverConfig, DohDnsTxtResolver, DohDnsTxtResolverConfig,
};
use std::sync::Arc;

/// DNS-over-HTTPS service for handles verified by a DNS record.
const DOH_URL: &str = "https://cloudflare-dns.com/dns-query";
/// Bluesky's public API, asked when a handle can't be checked directly.
const APPVIEW_URL: &str = "https://public.api.bsky.app";

/// The handle lookup browser sign-in uses.
pub type HandleResolver =
    Fallback<AtprotoHandleResolver<DohDnsTxtResolver<Xrpc>, Xrpc>, AppViewHandleResolver<Xrpc>>;

/// The AT Protocol's own checks first, then Bluesky's public API.
pub fn resolver(http: &Arc<Xrpc>) -> HandleResolver {
    Fallback::new(
        AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
            dns_txt_resolver: DohDnsTxtResolver::new(DohDnsTxtResolverConfig {
                service_url: DOH_URL.to_string(),
                http_client: http.clone(),
            }),
            http_client: http.clone(),
        }),
        AppViewHandleResolver::new(AppViewHandleResolverConfig {
            service_url: APPVIEW_URL.to_string(),
            http_client: http.clone(),
        }),
    )
}

/// Asks `first`, and `second` only when `first` has no answer.
pub struct Fallback<A, B> {
    first: A,
    second: B,
}

impl<A, B> Fallback<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}

impl<A, B> Resolver for Fallback<A, B>
where
    A: atrium_identity::handle::HandleResolver + Send + Sync,
    B: atrium_identity::handle::HandleResolver + Send + Sync,
{
    type Input = Handle;
    type Output = Did;
    type Error = Error;

    async fn resolve(&self, handle: &Handle) -> Result<Did, Error> {
        match self.first.resolve(handle).await {
            Ok(did) => Ok(did),
            Err(_) => self.second.resolve(handle).await,
        }
    }
}

impl<A, B> atrium_identity::handle::HandleResolver for Fallback<A, B>
where
    A: atrium_identity::handle::HandleResolver + Send + Sync,
    B: atrium_identity::handle::HandleResolver + Send + Sync,
{
}

/// What someone typed, as something sign-in can look up: a handle, a DID or a server
/// address. A name without a dot is taken as a bsky.social handle, since that is where
/// most accounts are.
pub fn sign_in_input(typed: &str) -> Result<String, AuthError> {
    let input = typed.trim().trim_start_matches('@');
    if input.starts_with("did:") || input.starts_with("https://") {
        return Ok(input.to_string());
    }
    if input.contains('@') {
        return Err(AuthError::EmailForBrowser);
    }
    let input = input.to_lowercase();
    Ok(if input.contains('.') {
        input
    } else {
        format!("{input}.bsky.social")
    })
}

/// Why signing in could not start, worded for the person who typed `input`.
pub fn sign_in_error(input: &str, e: atrium_oauth::Error) -> AuthError {
    match e {
        // Not a handle at all, or no server knows it.
        atrium_oauth::Error::Identity(
            Error::AtIdentifier(_) | Error::NotFound | Error::HttpStatus(_),
        ) => AuthError::HandleNotFound(input.to_string()),
        other => AuthError::Transport(format!("could not start signing in: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atrium_api::xrpc::http::StatusCode;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Answers with `did`, or not at all, and counts how often it was asked.
    struct Fixed {
        did: Option<&'static str>,
        asked: AtomicUsize,
    }

    impl Fixed {
        fn new(did: Option<&'static str>) -> Self {
            Self {
                did,
                asked: AtomicUsize::new(0),
            }
        }
    }

    impl Resolver for Fixed {
        type Input = Handle;
        type Output = Did;
        type Error = Error;

        async fn resolve(&self, _: &Handle) -> Result<Did, Error> {
            self.asked.fetch_add(1, Ordering::SeqCst);
            match self.did {
                Some(did) => Ok(Did::new(did.to_string()).unwrap()),
                None => Err(Error::HttpStatus(StatusCode::BAD_REQUEST)),
            }
        }
    }

    impl atrium_identity::handle::HandleResolver for Fixed {}

    fn handle() -> Handle {
        Handle::new("chama.pro".into()).unwrap()
    }

    #[tokio::test]
    async fn the_direct_answer_wins_and_bluesky_is_not_asked() {
        let both = Fallback::new(Fixed::new(Some("did:plc:a")), Fixed::new(Some("did:plc:b")));
        assert_eq!(both.resolve(&handle()).await.unwrap().as_str(), "did:plc:a");
        assert_eq!(both.second.asked.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn bluesky_answers_when_the_handle_can_t_be_checked_directly() {
        let both = Fallback::new(Fixed::new(None), Fixed::new(Some("did:plc:b")));
        assert_eq!(both.resolve(&handle()).await.unwrap().as_str(), "did:plc:b");
    }

    #[tokio::test]
    async fn no_answer_anywhere_is_an_error() {
        let both = Fallback::new(Fixed::new(None), Fixed::new(None));
        assert!(both.resolve(&handle()).await.is_err());
    }

    #[test]
    fn typed_input_becomes_something_to_look_up() {
        for (typed, wanted) in [
            ("valason", "valason.bsky.social"),
            (" @Valason ", "valason.bsky.social"),
            ("valason.bsky.social", "valason.bsky.social"),
            ("Chama.Pro", "chama.pro"),
            (
                "did:plc:layszv36l2j6y7i3j2rj4p3f",
                "did:plc:layszv36l2j6y7i3j2rj4p3f",
            ),
            ("https://bsky.social", "https://bsky.social"),
        ] {
            assert_eq!(sign_in_input(typed).unwrap(), wanted, "{typed}");
        }
    }

    #[test]
    fn an_email_asks_for_the_handle_instead() {
        assert!(matches!(
            sign_in_input("me@example.com"),
            Err(AuthError::EmailForBrowser)
        ));
    }

    #[test]
    fn an_unknown_handle_says_so() {
        for e in [
            Error::HttpStatus(StatusCode::BAD_REQUEST),
            Error::AtIdentifier("Invalid handle".into()),
            Error::NotFound,
        ] {
            let err = sign_in_error("valason.bsky.app", atrium_oauth::Error::Identity(e));
            assert!(
                matches!(&err, AuthError::HandleNotFound(h) if h == "valason.bsky.app"),
                "{err:?}"
            );
        }
        let err = sign_in_error("a.test", atrium_oauth::Error::Authorize("down".into()));
        assert!(matches!(err, AuthError::Transport(_)), "{err:?}");
    }
}
