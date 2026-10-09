//! Small types shared across the model: pages, labels, record references, record keys.

use super::AtUri;
use serde::Deserialize;

/// A page of results plus the cursor for the next one.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub src: String,
    pub val: String,
    #[serde(default)]
    pub neg: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrongRef {
    pub uri: AtUri,
    pub cid: String,
}

/// Your own like, repost, follow or block: still being saved, or saved as the record
/// at this URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mine {
    Saving,
    Saved(AtUri),
}

impl Mine {
    /// The record, once it is saved.
    pub fn uri(&self) -> Option<&AtUri> {
        match self {
            Mine::Saving => None,
            Mine::Saved(uri) => Some(uri),
        }
    }
}

/// What the server sends is always a saved record.
impl<'de> Deserialize<'de> for Mine {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        AtUri::deserialize(d).map(Mine::Saved)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplyRef {
    pub root: StrongRef,
    pub parent: StrongRef,
}

/// A record key in the timestamp form the official app uses for preference items.
pub fn tid() -> String {
    const ALPHABET: &[u8] = b"234567abcdefghijklmnopqrstuvwxyz";
    let micros = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_micros() as u64);
    let mut n = (micros & ((1 << 53) - 1)) << 10 | u64::from(std::process::id() & 0x3ff);
    let mut out = [0u8; 13];
    for c in out.iter_mut().rev() {
        *c = ALPHABET[(n & 31) as usize];
        n >>= 5;
    }
    String::from_utf8_lossy(&out).into_owned()
}
