//! Secrets in the OS credential store: tokens and API keys. Values too big for one
//! credential entry are stored in pieces ([`set_chunked`]).

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("secure storage error: {0}")]
    Secret(String),
    #[error("account list error: {0}")]
    Index(String),
}

/// Key/value storage for secrets. Keys are short ASCII identifiers.
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError>;
    fn set(&self, key: &str, value: &[u8]) -> Result<(), StoreError>;
    /// Deleting a missing key is not an error.
    fn delete(&self, key: &str) -> Result<(), StoreError>;
}

/// OS credential store: Windows Credential Manager, macOS Keychain, or the
/// Secret Service (GNOME Keyring / KWallet) on Linux.
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, StoreError> {
        keyring::Entry::new(&self.service, key).map_err(|e| StoreError::Secret(e.to_string()))
    }
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::new("bifrost")
    }
}

impl SecretStore for KeyringStore {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        match self.entry(key)?.get_secret() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(StoreError::Secret(e.to_string())),
        }
    }

    // `set_secret` stores raw UTF-8 bytes. `set_password` would store UTF-16 on
    // Windows and halve the 2560-byte credential limit.
    fn set(&self, key: &str, value: &[u8]) -> Result<(), StoreError> {
        self.entry(key)?
            .set_secret(value)
            .map_err(|e| StoreError::Secret(e.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), StoreError> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(StoreError::Secret(e.to_string())),
        }
    }
}

/// In-memory store for tests and ephemeral sessions.
#[derive(Default)]
pub struct MemoryStore {
    map: Mutex<HashMap<String, Vec<u8>>>,
}

impl SecretStore for MemoryStore {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self
            .map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(key)
            .cloned())
    }
    fn set(&self, key: &str, value: &[u8]) -> Result<(), StoreError> {
        self.map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key.into(), value.to_vec());
        Ok(())
    }
    fn delete(&self, key: &str) -> Result<(), StoreError> {
        self.map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(key);
        Ok(())
    }
}

/// Largest piece a value is stored in. Windows Credential Manager holds at most 2560
/// bytes per entry, which an OAuth session can come close to.
const CHUNK: usize = 2000;

/// Where a chunked value's pieces are: the key's pointer entry holds "g<generation>:<count>"
/// (or just "<count>" for values written before generations), and piece `i` lives at
/// "<key>:g<generation>:<i>" (or "<key>:<i>").
struct Chunks {
    generation: Option<u64>,
    count: usize,
}

impl Chunks {
    fn read(store: &dyn SecretStore, key: &str) -> Result<Option<Self>, StoreError> {
        let Some(raw) = store.get(key)? else {
            return Ok(None);
        };
        let raw = String::from_utf8_lossy(&raw);
        let bad = || StoreError::Secret(format!("{key}: bad chunk count"));
        Ok(Some(
            match raw.strip_prefix('g').and_then(|r| r.split_once(':')) {
                Some((g, n)) => Self {
                    generation: Some(g.parse().map_err(|_| bad())?),
                    count: n.parse().map_err(|_| bad())?,
                },
                None => Self {
                    generation: None,
                    count: raw.parse().map_err(|_| bad())?,
                },
            },
        ))
    }

    fn part(&self, key: &str, i: usize) -> String {
        match self.generation {
            Some(g) => format!("{key}:g{g}:{i}"),
            None => format!("{key}:{i}"),
        }
    }

    fn delete_parts(&self, store: &dyn SecretStore, key: &str) -> Result<(), StoreError> {
        for i in 0..self.count {
            store.delete(&self.part(key, i))?;
        }
        Ok(())
    }
}

/// Reads a value written by [`set_chunked`].
pub fn get_chunked(store: &dyn SecretStore, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
    let Some(chunks) = Chunks::read(store, key)? else {
        return Ok(None);
    };
    let mut out = Vec::new();
    for i in 0..chunks.count {
        match store.get(&chunks.part(key, i))? {
            Some(part) => out.extend(part),
            None => return Ok(None),
        }
    }
    Ok(Some(out))
}

/// Stores `value` in pieces small enough for any credential store. The pieces go
/// under a new generation and the pointer is switched last, so a crash or error
/// part way leaves the previous value whole.
pub fn set_chunked(store: &dyn SecretStore, key: &str, value: &[u8]) -> Result<(), StoreError> {
    let old = Chunks::read(store, key).ok().flatten();
    let new = Chunks {
        generation: Some(old.as_ref().and_then(|o| o.generation).map_or(1, |g| g + 1)),
        count: value.chunks(CHUNK).count(),
    };
    for (i, part) in value.chunks(CHUNK).enumerate() {
        store.set(&new.part(key, i), part)?;
    }
    let pointer = format!("g{}:{}", new.generation.unwrap_or(1), new.count);
    store.set(key, pointer.as_bytes())?;
    if let Some(old) = old {
        // Leftovers are harmless if this fails: nothing points at them any more.
        let _ = old.delete_parts(store, key);
    }
    Ok(())
}

pub fn delete_chunked(store: &dyn SecretStore, key: &str) -> Result<(), StoreError> {
    if let Some(chunks) = Chunks::read(store, key).ok().flatten() {
        chunks.delete_parts(store, key)?;
        // The generation before may be left over if replacing it failed part way.
        if let Some(g) = chunks.generation.filter(|g| *g > 1) {
            let before = Chunks {
                generation: Some(g - 1),
                count: chunks.count + 1,
            };
            // As above: nothing points at these any more.
            let _ = before.delete_parts(store, key);
        }
    }
    store.delete(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_roundtrip() {
        let s = MemoryStore::default();
        assert_eq!(s.get("k").unwrap(), None);
        s.set("k", b"v").unwrap();
        assert_eq!(s.get("k").unwrap().as_deref(), Some(&b"v"[..]));
        s.delete("k").unwrap();
        s.delete("k").unwrap();
        assert_eq!(s.get("k").unwrap(), None);
    }

    /// Talks to the real OS keyring, so it is opt-in: `cargo test -- --ignored`.
    #[test]
    #[ignore = "talks to the real OS keyring"]
    fn keyring_roundtrip() {
        let s = KeyringStore::new("bifrost-test");
        let key = format!("roundtrip-{}", std::process::id());
        s.set(&key, b"secret").unwrap();
        assert_eq!(s.get(&key).unwrap().as_deref(), Some(&b"secret"[..]));
        s.delete(&key).unwrap();
        assert_eq!(s.get(&key).unwrap(), None);
    }

    #[test]
    fn deleting_sweeps_a_generation_left_behind() {
        let s = MemoryStore::default();
        set_chunked(&s, "k", b"one").unwrap();
        set_chunked(&s, "k", b"two").unwrap();
        // As if replacing it couldn't delete the old piece.
        s.set("k:g1:0", b"one").unwrap();
        delete_chunked(&s, "k").unwrap();
        assert_eq!(s.get("k:g1:0").unwrap(), None);
        assert_eq!(s.get("k:g2:0").unwrap(), None);
    }

    #[test]
    fn chunked_secrets_roundtrip() {
        let store = MemoryStore::default();
        let big = vec![7u8; 4500];
        set_chunked(&store, "k", &big).unwrap();
        assert_eq!(get_chunked(&store, "k").unwrap(), Some(big));
        set_chunked(&store, "k", b"small").unwrap();
        assert_eq!(get_chunked(&store, "k").unwrap(), Some(b"small".to_vec()));
        // The first generation's pieces are gone.
        assert_eq!(store.get("k:g1:0").unwrap(), None);
        assert_eq!(store.get("k:g1:1").unwrap(), None);
        delete_chunked(&store, "k").unwrap();
        assert_eq!(get_chunked(&store, "k").unwrap(), None);
        assert_eq!(store.get("k:g2:0").unwrap(), None);

        // Values written before generations still read, and are replaced cleanly.
        store.set("old", b"2").unwrap();
        store.set("old:0", b"ab").unwrap();
        store.set("old:1", b"cd").unwrap();
        assert_eq!(get_chunked(&store, "old").unwrap(), Some(b"abcd".to_vec()));
        set_chunked(&store, "old", b"new").unwrap();
        assert_eq!(get_chunked(&store, "old").unwrap(), Some(b"new".to_vec()));
        assert_eq!(store.get("old:0").unwrap(), None);
    }
}
