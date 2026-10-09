use super::store::{AccountEntry, AccountIndex, IndexLocation};
use super::{AuthBackend, AuthError, Credentials, Session};
use crate::storage::secrets::{SecretStore, delete_chunked};
use std::time::Duration;
use tokio::sync::Mutex;

/// Refresh proactively when the access token has less than this left.
const DEFAULT_REFRESH_MARGIN: Duration = Duration::from_mins(2);
/// At startup, a session whose token has less than this left is checked (and
/// refreshed) before anything else.
const STARTUP_CHECK_MARGIN: Duration = Duration::from_mins(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSummary {
    pub did: String,
    pub handle: String,
    pub active: bool,
    /// Signed in through the browser rather than with an app password.
    pub oauth: bool,
}

/// Owns the account list and tokens. All session access goes through here so that
/// refreshed tokens are always written back to secure storage.
pub struct AccountManager<B, S> {
    backend: B,
    secrets: S,
    index: IndexLocation,
    refresh_margin: Duration,
    // Refresh tokens are single-use: two concurrent refreshes would revoke each other.
    refresh_lock: Mutex<()>,
    /// The account index and the active session, kept in memory so an API call
    /// doesn't read a file and the keyring each time. Written through on every
    /// change; the files stay the source of truth at startup.
    index_cache: std::sync::RwLock<Option<AccountIndex>>,
    active_cache: std::sync::RwLock<Option<Session>>,
}

impl<B: AuthBackend, S: SecretStore> AccountManager<B, S> {
    pub fn new(backend: B, secrets: S, index: IndexLocation) -> Self {
        Self {
            backend,
            secrets,
            index,
            refresh_margin: DEFAULT_REFRESH_MARGIN,
            refresh_lock: Mutex::new(()),
            index_cache: std::sync::RwLock::new(None),
            active_cache: std::sync::RwLock::new(None),
        }
    }

    #[must_use]
    pub fn with_refresh_margin(mut self, margin: Duration) -> Self {
        self.refresh_margin = margin;
        self
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Logs in, stores the session and makes it the active account.
    pub async fn login(&self, creds: &Credentials) -> Result<Session, AuthError> {
        let session = self.backend.login(creds).await?;
        self.persist(&session)?;
        let mut index = self.read_index()?;
        index.active = Some(session.did.clone());
        self.write_index(&index)?;
        Ok(session)
    }

    /// Records an account signed in through the browser and makes it active. Its
    /// tokens are kept by the OAuth client under [`oauth_key`].
    pub fn add_oauth(&self, did: &str, handle: &str, pds_url: &str) -> Result<(), AuthError> {
        let mut index = self.read_index()?;
        let entry = AccountEntry {
            did: did.into(),
            handle: handle.into(),
            pds_url: pds_url.into(),
            oauth: true,
        };
        match index.accounts.iter_mut().find(|a| a.did == did) {
            Some(existing) => *existing = entry,
            None => index.accounts.push(entry),
        }
        index.active = Some(did.into());
        self.write_index(&index)?;
        // Tokens from an earlier app-password login are no longer used.
        self.secrets.delete(&access_key(did))?;
        self.secrets.delete(&refresh_key(did))?;
        Ok(())
    }

    /// Whether `did` signs in through the browser.
    pub fn is_oauth(&self, did: &str) -> bool {
        self.with_index(|i| i.accounts.iter().any(|a| a.did == did && a.oauth))
            .unwrap_or(false)
    }

    /// The secret store, shared with the OAuth client.
    pub fn secrets(&self) -> &S {
        &self.secrets
    }

    pub fn accounts(&self) -> Result<Vec<AccountSummary>, AuthError> {
        self.with_index(|index| {
            index
                .accounts
                .iter()
                .map(|a| AccountSummary {
                    did: a.did.clone(),
                    handle: a.handle.clone(),
                    active: index.active.as_deref() == Some(a.did.as_str()),
                    oauth: a.oauth,
                })
                .collect()
        })
    }

    /// Makes `did_or_handle` the active account.
    pub fn switch(&self, did_or_handle: &str) -> Result<(), AuthError> {
        let mut index = self.read_index()?;
        let did = index
            .accounts
            .iter()
            .find(|a| a.did == did_or_handle || a.handle == did_or_handle)
            .map(|a| a.did.clone())
            .ok_or_else(|| AuthError::UnknownAccount(did_or_handle.into()))?;
        index.active = Some(did);
        self.write_index(&index)?;
        Ok(())
    }

    /// The active account's session, refreshed first if the access token is close to
    /// expiry. Cheap: no network call unless a refresh is due.
    pub async fn session(&self) -> Result<Session, AuthError> {
        let session = self.load_active()?;
        if !session.access_expires_within(self.refresh_margin) {
            return Ok(session);
        }
        self.refresh_did(&session.did, false).await
    }

    /// Saves a session whose tokens were rotated outside the manager (for example
    /// by an API client retrying after an expired token), but only if the stored
    /// tokens are still the ones that call started from: if something else has
    /// refreshed since, its newer tokens win. Ignored for unknown accounts.
    pub async fn update_session(
        &self,
        before: &Session,
        session: &Session,
    ) -> Result<(), AuthError> {
        let _guard = self.refresh_lock.lock().await;
        match self.load(&session.did)? {
            Some(current) if current.refresh_jwt == before.refresh_jwt && &current != session => {
                self.persist(session)
            }
            _ => Ok(()),
        }
    }

    /// Forces a token refresh for the active account.
    pub async fn refresh(&self) -> Result<Session, AuthError> {
        let current = self.load_active()?;
        // OAuth sessions refresh themselves.
        if self.is_oauth(&current.did) {
            return Ok(current);
        }
        self.refresh_did(&current.did, true).await
    }

    /// Startup path: checks the stored session with the server (refreshing if needed)
    /// and saves whatever comes back.
    pub async fn restore(&self) -> Result<Session, AuthError> {
        let _guard = self.refresh_lock.lock().await;
        let stored = self.load_active()?;
        // The OAuth client checks its own session on the first request. A token with
        // a while left needn't be checked either: the first request does that, and
        // starting up doesn't wait on an extra round trip.
        if self.is_oauth(&stored.did) || !stored.access_expires_within(STARTUP_CHECK_MARGIN) {
            return Ok(stored);
        }
        let result = self.backend.validate(&stored).await;
        self.settle(&stored, result)
    }

    /// Logs out `did` (or the active account), revoking server-side when possible and
    /// always removing local tokens.
    pub async fn logout(&self, did: Option<&str>) -> Result<(), AuthError> {
        // A refresh under way finishes first, so it can't write tokens back after.
        let _guard = self.refresh_lock.lock().await;
        let mut index = self.read_index()?;
        let did = match did {
            Some(d) => d.to_string(),
            None => index.active.clone().ok_or(AuthError::NoActiveAccount)?,
        };
        if let Some(session) = self.load(&did)?
            && !self.is_oauth(&did)
        {
            // Best effort: a dead network must not stop someone logging out.
            let _ = self.backend.logout(&session).await;
        }
        self.forget_tokens(&did)?;
        index.accounts.retain(|a| a.did != did);
        if index.active.as_deref() == Some(did.as_str()) {
            index.active = index.accounts.first().map(|a| a.did.clone());
        }
        self.write_index(&index)?;
        Ok(())
    }

    async fn refresh_did(&self, did: &str, force: bool) -> Result<Session, AuthError> {
        let _guard = self.refresh_lock.lock().await;
        // Another task may have refreshed while we waited for the lock.
        let current = self.load(did)?.ok_or(AuthError::SessionExpired)?;
        if !force && !current.access_expires_within(self.refresh_margin) {
            return Ok(current);
        }
        let result = self.backend.refresh(&current).await;
        self.settle(&current, result)
    }

    /// Saves a fresh session, or drops dead tokens so the UI can ask for a new login.
    fn settle(
        &self,
        old: &Session,
        result: Result<Session, AuthError>,
    ) -> Result<Session, AuthError> {
        match result {
            Ok(session) => {
                // An account signed out meanwhile stays signed out.
                let known = self.read_index()?.accounts.iter().any(|a| a.did == old.did);
                if !known {
                    return Err(AuthError::SessionExpired);
                }
                if &session != old {
                    self.persist(&session)?;
                }
                Ok(session)
            }
            Err(AuthError::SessionExpired) => {
                self.forget_tokens(&old.did)?;
                Err(AuthError::SessionExpired)
            }
            Err(e) => Err(e),
        }
    }

    fn load_active(&self) -> Result<Session, AuthError> {
        if let Some(s) = self
            .active_cache
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        {
            return Ok(s);
        }
        let did = self
            .with_index(|i| i.active.clone())?
            .ok_or(AuthError::NoActiveAccount)?;
        let session = self.load(&did)?.ok_or(AuthError::SessionExpired)?;
        *self
            .active_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(session.clone());
        Ok(session)
    }

    /// A copy of the account index, to change and write back.
    fn read_index(&self) -> Result<AccountIndex, AuthError> {
        self.with_index(AccountIndex::clone)
    }

    /// Looks at the account index without copying it, reading the file only the
    /// first time.
    fn with_index<T>(&self, f: impl FnOnce(&AccountIndex) -> T) -> Result<T, AuthError> {
        if let Some(i) = self
            .index_cache
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return Ok(f(i));
        }
        let index = self.index.load()?;
        let out = f(&index);
        *self
            .index_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(index);
        Ok(out)
    }

    fn write_index(&self, index: &AccountIndex) -> Result<(), AuthError> {
        self.index.save(index)?;
        *self
            .index_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(index.clone());
        // Which account is active may have changed: look it up again next time.
        let mut active = self
            .active_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if active.as_ref().map(|s| &s.did) != index.active.as_ref() {
            *active = None;
        }
        Ok(())
    }

    fn load(&self, did: &str) -> Result<Option<Session>, AuthError> {
        let Some(entry) = self.with_index(|i| i.accounts.iter().find(|a| a.did == did).cloned())?
        else {
            return Ok(None);
        };
        if entry.oauth {
            // The tokens live with the OAuth client; this only says who is signed in.
            return Ok(self.secrets.get(&oauth_key(did))?.map(|_| Session {
                did: entry.did.clone(),
                handle: entry.handle.clone(),
                pds_url: entry.pds_url.clone(),
                access_jwt: String::new(),
                refresh_jwt: String::new(),
            }));
        }
        let (Some(access), Some(refresh)) = (
            self.secrets.get(&access_key(did))?,
            self.secrets.get(&refresh_key(did))?,
        ) else {
            return Ok(None);
        };
        Ok(Some(Session {
            did: entry.did.clone(),
            handle: entry.handle.clone(),
            pds_url: entry.pds_url.clone(),
            access_jwt: String::from_utf8_lossy(&access).into_owned(),
            refresh_jwt: String::from_utf8_lossy(&refresh).into_owned(),
        }))
    }

    fn persist(&self, session: &Session) -> Result<(), AuthError> {
        // Refresh token first: if we crash in between, the old access token simply
        // expires and the new refresh token still works.
        self.secrets
            .set(&refresh_key(&session.did), session.refresh_jwt.as_bytes())?;
        self.secrets
            .set(&access_key(&session.did), session.access_jwt.as_bytes())?;
        let mut index = self.read_index()?;
        let entry = AccountEntry {
            did: session.did.clone(),
            handle: session.handle.clone(),
            pds_url: session.pds_url.clone(),
            oauth: false,
        };
        match index.accounts.iter_mut().find(|a| a.did == session.did) {
            Some(existing) => *existing = entry,
            None => index.accounts.push(entry),
        }
        self.write_index(&index)?;
        let mut active = self
            .active_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if active.as_ref().is_some_and(|a| a.did == session.did) {
            *active = Some(session.clone());
        }
        Ok(())
    }

    fn forget_tokens(&self, did: &str) -> Result<(), AuthError> {
        let mut active = self
            .active_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if active.as_ref().is_some_and(|a| a.did == did) {
            *active = None;
        }
        drop(active);
        self.secrets.delete(&access_key(did))?;
        self.secrets.delete(&refresh_key(did))?;
        delete_chunked(&self.secrets, &oauth_key(did))?;
        Ok(())
    }
}

/// Where an OAuth session for `did` is stored, in chunks.
pub fn oauth_key(did: &str) -> String {
    format!("{did}:oauth")
}

fn access_key(did: &str) -> String {
    format!("{did}:access")
}

fn refresh_key(did: &str) -> String {
    format!("{did}:refresh")
}

#[cfg(test)]
mod tests;
