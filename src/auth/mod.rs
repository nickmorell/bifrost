//! Authentication and session persistence.
//!
//! The rest of the app only talks to the types in this module:
//! - [`AuthBackend`]: the network side (login, refresh, validate, logout). The current
//!   implementation is [`atrium::AtriumBackend`]; a custom client can replace it later
//!   without touching callers.
//! - [`SecretStore`](crate::storage::secrets::SecretStore): where tokens live.
//!   `KeyringStore` is the default and keeps tokens in the OS credential store, never
//!   on disk in plaintext.
//! - [`AccountManager`]: ties the two together, handles multiple accounts and keeps
//!   tokens fresh.

pub mod atrium;
mod manager;
pub mod oauth;
mod sessions;
pub mod store;
pub mod xrpc;

use crate::storage::secrets::StoreError;
pub use manager::{AccountManager, AccountSummary, oauth_key};

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The default entryway for Bluesky-hosted accounts. Login is resolved to the
/// account's own PDS afterwards.
pub const DEFAULT_SERVICE: &str = "https://bsky.social";

/// An authenticated AT Protocol session. Backend-agnostic.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub did: String,
    pub handle: String,
    /// The account's PDS. API calls for this account should go here.
    pub pds_url: String,
    pub access_jwt: String,
    pub refresh_jwt: String,
}

impl Session {
    /// Expiry of the access token, read from its `exp` claim. `None` if the token
    /// is not a JWT we can read, in which case callers should rely on server errors.
    pub fn access_expires_at(&self) -> Option<SystemTime> {
        jwt_exp(&self.access_jwt)
    }

    /// True if the access token expires within `margin` (or already has).
    pub fn access_expires_within(&self, margin: Duration) -> bool {
        match self.access_expires_at() {
            Some(exp) => exp <= SystemTime::now() + margin,
            None => false,
        }
    }
}

// Tokens must never end up in logs.
impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("did", &self.did)
            .field("handle", &self.handle)
            .field("pds_url", &self.pds_url)
            .field("access_jwt", &"<redacted>")
            .field("refresh_jwt", &"<redacted>")
            .finish()
    }
}

/// Login input. Use an app password, not the account's main password.
pub struct Credentials {
    /// Handle, DID or email.
    pub identifier: String,
    pub password: String,
    /// Email 2FA code, when the server asks for one.
    pub auth_factor_token: Option<String>,
    /// Entryway to log in through, e.g. [`DEFAULT_SERVICE`].
    pub service: String,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("identifier", &self.identifier)
            .field("password", &"<redacted>")
            .field("service", &self.service)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid identifier or password")]
    InvalidCredentials,
    #[error("a 2FA code was sent to the account email; log in again with it")]
    AuthFactorRequired,
    #[error("session expired; log in again")]
    SessionExpired,
    #[error("account is taken down or deactivated")]
    AccountUnavailable,
    #[error("no account is logged in")]
    NoActiveAccount,
    #[error("unknown account: {0}")]
    UnknownAccount(String),
    #[error("network or server error: {0}")]
    Transport(String),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Network side of authentication. Implementations must be stateless with respect to
/// persistence: they return updated sessions and the caller decides what to store.
#[expect(
    async_fn_in_trait,
    reason = "BiFrost is the only implementer and caller, so no caller needs a `Send` bound spelled out"
)]
pub trait AuthBackend {
    async fn login(&self, creds: &Credentials) -> Result<Session, AuthError>;
    /// Exchange the refresh token for a new token pair.
    async fn refresh(&self, session: &Session) -> Result<Session, AuthError>;
    /// Check the session against the server, refreshing if the access token has
    /// expired. Returns the (possibly updated) session.
    async fn validate(&self, session: &Session) -> Result<Session, AuthError>;
    /// Revoke the session server-side.
    async fn logout(&self, session: &Session) -> Result<(), AuthError>;
}

fn jwt_exp(token: &str) -> Option<SystemTime> {
    #[derive(Deserialize)]
    struct Claims {
        exp: u64,
    }
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    let claims: Claims = serde_json::from_slice(&bytes).ok()?;
    Some(UNIX_EPOCH + Duration::from_secs(claims.exp))
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;

    /// Builds an unsigned JWT-shaped token with the given `exp`. Good enough for
    /// claim parsing; never sent anywhere.
    pub fn fake_jwt(exp: SystemTime) -> String {
        let enc = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let exp = exp.duration_since(UNIX_EPOCH).unwrap().as_secs();
        format!(
            "{}.{}.sig",
            enc.encode(br#"{"alg":"ES256K","typ":"JWT"}"#),
            enc.encode(format!(r#"{{"exp":{exp},"sub":"did:plc:test"}}"#))
        )
    }

    pub fn session(did: &str, access_exp: SystemTime) -> Session {
        Session {
            did: did.into(),
            handle: format!("{}.test", did.rsplit(':').next().unwrap()),
            pds_url: "https://pds.example".into(),
            access_jwt: fake_jwt(access_exp),
            refresh_jwt: "refresh-".to_string() + did,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;

    #[test]
    fn reads_access_expiry() {
        let in_an_hour = SystemTime::now() + Duration::from_hours(1);
        let s = session("did:plc:a", in_an_hour);
        assert!(!s.access_expires_within(Duration::from_mins(1)));
        assert!(s.access_expires_within(Duration::from_hours(2)));
    }

    #[test]
    fn unreadable_token_is_not_treated_as_expired() {
        let mut s = session("did:plc:a", SystemTime::now());
        s.access_jwt = "opaque".into();
        assert_eq!(s.access_expires_at(), None);
        assert!(!s.access_expires_within(Duration::from_mins(1)));
    }

    #[test]
    fn debug_redacts_tokens() {
        let s = session("did:plc:a", SystemTime::now());
        let out = format!("{s:?}");
        assert!(!out.contains(&s.access_jwt));
        assert!(!out.contains(&s.refresh_jwt));
    }
}
