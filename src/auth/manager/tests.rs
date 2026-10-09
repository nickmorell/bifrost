//! The account manager against a scripted backend and an in-memory keyring.

use super::*;
use crate::auth::test_util::{fake_jwt, session};
use crate::storage::secrets::MemoryStore;
use std::{sync::Mutex as StdMutex, time::SystemTime};

/// Scripted backend that counts calls.
#[derive(Default)]
struct FakeBackend {
    refresh_fails: bool,
    validate_fails: bool,
    /// Refreshing takes a moment, so something else can happen meanwhile.
    slow_refresh: bool,
    calls: StdMutex<Vec<&'static str>>,
}

impl FakeBackend {
    fn calls(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().clone()
    }
}

fn later() -> SystemTime {
    SystemTime::now() + Duration::from_hours(1)
}

impl AuthBackend for FakeBackend {
    async fn login(&self, creds: &Credentials) -> Result<Session, AuthError> {
        self.calls.lock().unwrap().push("login");
        if creds.password != "app-pass" {
            return Err(AuthError::InvalidCredentials);
        }
        Ok(session(&creds.identifier, later()))
    }
    async fn refresh(&self, s: &Session) -> Result<Session, AuthError> {
        self.calls.lock().unwrap().push("refresh");
        if self.slow_refresh {
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        if self.refresh_fails {
            return Err(AuthError::SessionExpired);
        }
        Ok(Session {
            access_jwt: fake_jwt(later()),
            refresh_jwt: s.refresh_jwt.clone() + "+",
            ..s.clone()
        })
    }
    async fn validate(&self, s: &Session) -> Result<Session, AuthError> {
        self.calls.lock().unwrap().push("validate");
        if self.validate_fails {
            return Err(AuthError::SessionExpired);
        }
        Ok(s.clone())
    }
    async fn logout(&self, _: &Session) -> Result<(), AuthError> {
        self.calls.lock().unwrap().push("logout");
        Err(AuthError::Transport("offline".into()))
    }
}

fn manager(backend: FakeBackend) -> AccountManager<FakeBackend, MemoryStore> {
    AccountManager::new(backend, MemoryStore::default(), IndexLocation::memory())
}

fn creds(did: &str) -> Credentials {
    Credentials {
        identifier: did.into(),
        password: "app-pass".into(),
        auth_factor_token: None,
        service: "https://entryway.example".into(),
    }
}

/// Overwrites the stored access token of `did` with one expiring at `exp`.
fn set_access_exp<B: AuthBackend>(m: &AccountManager<B, MemoryStore>, did: &str, exp: SystemTime) {
    m.secrets
        .set(&access_key(did), fake_jwt(exp).as_bytes())
        .unwrap();
    // Written behind the manager's back, so drop what it remembers.
    *m.active_cache.write().unwrap() = None;
}

#[tokio::test]
async fn login_persists_and_activates() {
    let m = manager(FakeBackend::default());
    let s = m.login(&creds("did:plc:a")).await.unwrap();
    assert_eq!(m.session().await.unwrap(), s);
    assert_eq!(
        m.accounts().unwrap(),
        vec![AccountSummary {
            did: "did:plc:a".into(),
            handle: "a.test".into(),
            active: true,
            oauth: false,
        }]
    );
    assert_eq!(m.backend().calls(), ["login"]);
}

#[tokio::test]
async fn bad_password_stores_nothing() {
    let m = manager(FakeBackend::default());
    let mut c = creds("did:plc:a");
    c.password = "wrong".into();
    assert!(matches!(
        m.login(&c).await,
        Err(AuthError::InvalidCredentials)
    ));
    assert!(m.accounts().unwrap().is_empty());
    assert!(matches!(m.session().await, Err(AuthError::NoActiveAccount)));
}

#[tokio::test]
async fn fresh_token_skips_refresh() {
    let m = manager(FakeBackend::default());
    m.login(&creds("did:plc:a")).await.unwrap();
    m.session().await.unwrap();
    assert_eq!(m.backend().calls(), ["login"]);
}

#[tokio::test]
async fn expiring_token_is_refreshed_and_saved() {
    let m = manager(FakeBackend::default());
    m.login(&creds("did:plc:a")).await.unwrap();
    set_access_exp(&m, "did:plc:a", SystemTime::now() + Duration::from_secs(10));

    let s = m.session().await.unwrap();
    assert_eq!(s.refresh_jwt, "refresh-did:plc:a+");
    assert!(!s.access_expires_within(Duration::from_mins(10)));
    // The rotated tokens were written back to the store.
    assert_eq!(m.load("did:plc:a").unwrap().unwrap(), s);
    assert_eq!(m.backend().calls(), ["login", "refresh"]);
}

#[tokio::test]
async fn failed_refresh_drops_tokens_but_keeps_account() {
    let m = manager(FakeBackend {
        refresh_fails: true,
        ..Default::default()
    });
    m.login(&creds("did:plc:a")).await.unwrap();
    set_access_exp(&m, "did:plc:a", SystemTime::now());

    assert!(matches!(m.session().await, Err(AuthError::SessionExpired)));
    assert_eq!(m.secrets.get(&refresh_key("did:plc:a")).unwrap(), None);
    // Still listed, so the UI can offer "log in again as a.test".
    assert_eq!(m.accounts().unwrap().len(), 1);
}

#[tokio::test]
async fn restore_validates_with_server() {
    // A token with plenty of time left is used as it is.
    let m = manager(FakeBackend::default());
    m.login(&creds("did:plc:a")).await.unwrap();
    set_access_exp(&m, "did:plc:a", SystemTime::now() + Duration::from_hours(1));
    m.restore().await.unwrap();
    assert_eq!(m.backend().calls(), ["login"]);

    // One about to run out is checked first.
    let m = manager(FakeBackend::default());
    m.login(&creds("did:plc:a")).await.unwrap();
    set_access_exp(&m, "did:plc:a", SystemTime::now() + Duration::from_mins(1));
    m.restore().await.unwrap();
    assert_eq!(m.backend().calls(), ["login", "validate"]);

    let m = manager(FakeBackend {
        validate_fails: true,
        ..Default::default()
    });
    m.login(&creds("did:plc:a")).await.unwrap();
    set_access_exp(&m, "did:plc:a", SystemTime::now() + Duration::from_mins(1));
    assert!(matches!(m.restore().await, Err(AuthError::SessionExpired)));
}

#[tokio::test]
async fn multiple_accounts_switch_and_logout() {
    let m = manager(FakeBackend::default());
    m.login(&creds("did:plc:a")).await.unwrap();
    m.login(&creds("did:plc:b")).await.unwrap();
    assert_eq!(m.session().await.unwrap().did, "did:plc:b");

    m.switch("a.test").unwrap();
    assert_eq!(m.session().await.unwrap().did, "did:plc:a");
    assert!(matches!(
        m.switch("nobody"),
        Err(AuthError::UnknownAccount(_))
    ));

    // Server revoke fails (offline) but local logout still succeeds.
    m.logout(None).await.unwrap();
    assert_eq!(m.secrets.get(&access_key("did:plc:a")).unwrap(), None);
    assert_eq!(m.session().await.unwrap().did, "did:plc:b");
    let accounts = m.accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    assert!(accounts[0].active);
}

#[tokio::test]
async fn logging_out_during_a_refresh_stays_logged_out() {
    let m = manager(FakeBackend {
        slow_refresh: true,
        ..Default::default()
    });
    m.login(&creds("did:plc:a")).await.unwrap();
    let logout = async {
        tokio::task::yield_now().await;
        m.logout(Some("did:plc:a")).await
    };
    let (_, out) = tokio::join!(m.refresh(), logout);
    out.unwrap();
    assert!(m.accounts().unwrap().is_empty());
    assert_eq!(m.secrets.get(&refresh_key("did:plc:a")).unwrap(), None);
    assert_eq!(m.secrets.get(&access_key("did:plc:a")).unwrap(), None);
}
