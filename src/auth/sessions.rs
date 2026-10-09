//! Where the OAuth client keeps its sessions (a DPoP key and tokens): the OS
//! credential store, one entry per account, in pieces.

use super::oauth_key;
use crate::storage::secrets::{SecretStore, StoreError, delete_chunked, get_chunked, set_chunked};
use atrium_api::types::string::Did;
use atrium_common::store::Store;
use atrium_oauth::store::session::{Session as OAuthTokens, SessionStore};
use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};

/// OAuth sessions in the OS credential store. Cloning shares the same store and the
/// same list of accounts signed out.
#[derive(Clone)]
pub struct Sessions {
    secrets: Arc<dyn SecretStore>,
    /// Accounts signed out since the last sign-in. A refresh already under way when
    /// someone signs out would otherwise write their tokens back afterwards.
    signed_out: Arc<Mutex<HashSet<String>>>,
}

impl Sessions {
    pub fn new(secrets: Arc<dyn SecretStore>) -> Self {
        Self {
            secrets,
            signed_out: Arc::default(),
        }
    }

    /// Deletes `did`'s session and keeps it from being written again until the next
    /// sign-in.
    pub fn sign_out(&self, did: &str) -> Result<(), StoreError> {
        self.signed_out
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(did.to_string());
        delete_chunked(self.secrets.as_ref(), &oauth_key(did))
    }

    /// A sign-in is starting: whoever it is for may be kept again.
    pub fn signing_in(&self) {
        self.signed_out
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    fn is_signed_out(&self, did: &str) -> bool {
        self.signed_out
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(did)
    }
}

impl Store<Did, OAuthTokens> for Sessions {
    type Error = StoreError;

    async fn get(&self, key: &Did) -> Result<Option<OAuthTokens>, StoreError> {
        match get_chunked(self.secrets.as_ref(), &oauth_key(key.as_str()))? {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| StoreError::Secret(e.to_string())),
            None => Ok(None),
        }
    }

    async fn set(&self, key: Did, value: OAuthTokens) -> Result<(), StoreError> {
        if self.is_signed_out(key.as_str()) {
            return Ok(());
        }
        let bytes = serde_json::to_vec(&value).map_err(|e| StoreError::Secret(e.to_string()))?;
        set_chunked(self.secrets.as_ref(), &oauth_key(key.as_str()), &bytes)
    }

    async fn del(&self, key: &Did) -> Result<(), StoreError> {
        delete_chunked(self.secrets.as_ref(), &oauth_key(key.as_str()))
    }

    async fn clear(&self) -> Result<(), StoreError> {
        // Sessions are only ever removed one account at a time.
        Ok(())
    }
}

impl SessionStore for Sessions {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::secrets::MemoryStore;
    use serde_json::json;

    fn tokens(did: &str) -> OAuthTokens {
        serde_json::from_value(json!({
            "dpop_key": {"kty": "oct", "k": "c2VjcmV0"},
            "token_set": {
                "iss": "https://bsky.social", "sub": did, "aud": "https://pds.example",
                "scope": null, "refresh_token": "r", "access_token": "a",
                "token_type": "DPoP", "expires_at": null,
            },
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn a_refresh_after_signing_out_keeps_nothing() {
        let store: Arc<dyn SecretStore> = Arc::new(MemoryStore::default());
        let sessions = Sessions::new(store);
        let did = Did::new("did:plc:a".into()).unwrap();
        sessions
            .set(did.clone(), tokens("did:plc:a"))
            .await
            .unwrap();
        assert!(sessions.get(&did).await.unwrap().is_some());

        sessions.sign_out("did:plc:a").unwrap();
        // A refresh that was under way finishes and saves its new tokens.
        sessions
            .set(did.clone(), tokens("did:plc:a"))
            .await
            .unwrap();
        assert!(sessions.get(&did).await.unwrap().is_none());

        sessions.signing_in();
        sessions
            .set(did.clone(), tokens("did:plc:a"))
            .await
            .unwrap();
        assert!(sessions.get(&did).await.unwrap().is_some());
    }
}
