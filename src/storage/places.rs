//! Where you were reading in each feed, kept between runs: the entry at the top of
//! the view, per account and feed.

use super::later::Later;
use crate::model::Did;
use crate::platform::paths;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Places {
    /// By account, then by feed (`FeedSource::id`): the entry at the top of the view.
    by_account: BTreeMap<Did, BTreeMap<String, String>>,
}

impl Places {
    fn path() -> Option<PathBuf> {
        Some(paths::config_dir()?.join("places.json"))
    }

    /// Loads the places, or none if the file is missing or unreadable.
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Saves the places in the background, or before returning when BiFrost is about
    /// to quit (`now`).
    pub fn save(&self, now: bool) {
        static FILE: Later = Later::new("your reading places");
        if let (Some(path), Ok(json)) = (Self::path(), serde_json::to_vec(self)) {
            if now {
                FILE.write_now(&path, &json);
            } else {
                FILE.write(path, json);
            }
        }
    }

    /// The entry at the top of `feed` when `account` last read it.
    pub fn get(&self, account: &str, feed: &str) -> Option<&str> {
        self.by_account.get(account)?.get(feed).map(String::as_str)
    }

    /// Remembers the entry at the top of `feed`. Says whether it changed.
    pub fn set(&mut self, account: &Did, feed: &str, item: &str) -> bool {
        let feeds = self.by_account.entry(account.clone()).or_default();
        if feeds.get(feed).map(String::as_str) == Some(item) {
            return false;
        }
        feeds.insert(feed.to_string(), item.to_string());
        true
    }

    /// Drops what belongs to one account, when it logs out.
    pub fn forget_account(&mut self, account: &str) {
        self.by_account.remove(account);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_are_kept_per_account_and_feed() {
        let mut p = Places::default();
        let a = Did::from("did:plc:a");
        assert!(p.set(&a, "following", "at://x/1"));
        assert!(!p.set(&a, "following", "at://x/1"), "unchanged");
        assert_eq!(p.get("did:plc:a", "following"), Some("at://x/1"));
        assert_eq!(p.get("did:plc:b", "following"), None);
        let json = serde_json::to_vec(&p).unwrap();
        let back: Places = serde_json::from_slice(&json).unwrap();
        assert_eq!(back.get("did:plc:a", "following"), Some("at://x/1"));
        p.forget_account("did:plc:a");
        assert_eq!(p.get("did:plc:a", "following"), None);
    }
}
