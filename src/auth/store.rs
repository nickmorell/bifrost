//! The account list (not secret), stored as a file beside the settings. Tokens are
//! kept in [`crate::storage::secrets`].

use crate::platform::paths;
use crate::storage::secrets::StoreError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Non-secret account metadata. Kept out of the keyring so the list can grow
/// without hitting per-credential size limits.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountIndex {
    pub accounts: Vec<AccountEntry>,
    pub active: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountEntry {
    pub did: String,
    pub handle: String,
    pub pds_url: String,
    /// Signed in through the browser (OAuth) rather than with an app password.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub oauth: bool,
}

/// Where the [`AccountIndex`] lives.
pub enum IndexLocation {
    File(PathBuf),
    Memory(Mutex<AccountIndex>),
}

impl IndexLocation {
    /// `<config dir>/bifrost/accounts.json`, e.g. `%APPDATA%\bifrost\accounts.json`.
    pub fn default_file() -> Result<Self, StoreError> {
        let dir = paths::config_dir()
            .ok_or_else(|| StoreError::Index("no config directory on this system".into()))?;
        Ok(Self::File(dir.join("accounts.json")))
    }

    pub fn memory() -> Self {
        Self::Memory(Mutex::default())
    }

    pub fn load(&self) -> Result<AccountIndex, StoreError> {
        match self {
            Self::Memory(m) => Ok(m
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()),
            Self::File(path) => match std::fs::read(path) {
                Ok(bytes) => {
                    serde_json::from_slice(&bytes).map_err(|e| StoreError::Index(e.to_string()))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AccountIndex::default()),
                Err(e) => Err(StoreError::Index(e.to_string())),
            },
        }
    }

    pub fn save(&self, index: &AccountIndex) -> Result<(), StoreError> {
        match self {
            Self::Memory(m) => {
                *m.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = index.clone();
                Ok(())
            }
            Self::File(path) => {
                let io = |e: std::io::Error| StoreError::Index(e.to_string());
                // With nobody signed in there is nothing to keep, not even who was.
                if index.accounts.is_empty() {
                    return match std::fs::remove_file(path) {
                        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(io(e)),
                        _ => Ok(()),
                    };
                }
                if let Some(dir) = path.parent() {
                    paths::create_dir(dir).map_err(io)?;
                }
                let json = serde_json::to_vec_pretty(index)
                    .map_err(|e| StoreError::Index(e.to_string()))?;
                // Write then rename so a crash never leaves a half-written index.
                let tmp = path.with_extension("json.tmp");
                std::fs::write(&tmp, json).map_err(io)?;
                std::fs::rename(&tmp, path).map_err(io)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!("bifrost-test-{}", std::process::id()));
        let loc = IndexLocation::File(dir.join("accounts.json"));
        assert_eq!(loc.load().unwrap(), AccountIndex::default());
        let idx = AccountIndex {
            accounts: vec![AccountEntry {
                oauth: false,
                did: "did:plc:a".into(),
                handle: "a.test".into(),
                pds_url: "https://pds.example".into(),
            }],
            active: Some("did:plc:a".into()),
        };
        loc.save(&idx).unwrap();
        assert_eq!(loc.load().unwrap(), idx);
        // The last account signing out leaves no file behind.
        loc.save(&AccountIndex::default()).unwrap();
        assert!(!dir.join("accounts.json").exists());
        assert_eq!(loc.load().unwrap(), AccountIndex::default());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
