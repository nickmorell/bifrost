//! A post being written that was kept for later: what the Drafts screen lists.
//! `storage::drafts` keeps them on this computer.

use super::AtUri;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    /// Milliseconds since the epoch when first saved; unique per draft.
    pub id: u64,
    /// DID of the account that wrote it.
    pub account: String,
    /// RFC 3339 time of the last save.
    pub saved_at: String,
    pub text: String,
    /// URI of the post being replied to or quoted, if any.
    #[serde(default)]
    pub reply_to: Option<AtUri>,
    #[serde(default)]
    pub quote: Option<AtUri>,
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub images: Vec<DraftImageFile>,
    /// When to send it (RFC 3339), for a scheduled post.
    #[serde(default)]
    pub send_at: Option<String>,
}

impl Draft {
    /// Scheduled for `now` or earlier.
    pub fn due(&self, now: &chrono::DateTime<chrono::Utc>) -> bool {
        self.send_at
            .as_deref()
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .is_some_and(|t| t <= *now)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftImageFile {
    pub file: String,
    pub alt: String,
    pub width: u32,
    pub height: u32,
}
