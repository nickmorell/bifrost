//! Your collections of posts, kept on this computer per account.

use super::later::Later;
use crate::model::{AtUri, Collection, Did, collection_name};
use crate::platform::paths;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Collections {
    by_account: BTreeMap<Did, Vec<Collection>>,
}

impl Collections {
    fn path() -> Option<PathBuf> {
        Some(paths::config_dir()?.join("collections.json"))
    }

    /// Loads the collections, or none if the file is missing or unreadable.
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Saves them in the background.
    pub fn save(&self) {
        static FILE: Later = Later::new("your collections");
        if let (Some(path), Ok(json)) = (Self::path(), serde_json::to_vec(self)) {
            FILE.write(path, json);
        }
    }

    /// An account's collections, newest first.
    pub fn list(&self, account: &str) -> &[Collection] {
        self.by_account.get(account).map_or(&[], Vec::as_slice)
    }

    /// Makes a collection, with `first` in it if given. Gives its id, or `None` when
    /// the name is blank.
    pub fn create(&mut self, account: &Did, name: &str, first: Option<AtUri>) -> Option<u64> {
        let name = collection_name(name)?;
        let list = self.by_account.entry(account.clone()).or_default();
        let now = u64::try_from(chrono::Utc::now().timestamp_millis()).unwrap_or_default();
        let id = list.iter().map(|c| c.id + 1).fold(now, u64::max);
        list.insert(
            0,
            Collection {
                id,
                name,
                posts: first.into_iter().collect(),
            },
        );
        Some(id)
    }

    pub fn rename(&mut self, account: &str, id: u64, name: &str) {
        if let (Some(c), Some(name)) = (self.find(account, id), collection_name(name)) {
            c.name = name;
        }
    }

    pub fn delete(&mut self, account: &str, id: u64) {
        if let Some(list) = self.by_account.get_mut(account) {
            list.retain(|c| c.id != id);
        }
    }

    /// Adds a post to a collection, or takes it out. Says whether it is in now.
    pub fn toggle(&mut self, account: &str, id: u64, uri: &AtUri) -> bool {
        self.find(account, id).is_some_and(|c| c.toggle(uri))
    }

    fn find(&mut self, account: &str, id: u64) -> Option<&mut Collection> {
        self.by_account
            .get_mut(account)?
            .iter_mut()
            .find(|c| c.id == id)
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
    fn collections_belong_to_one_account() {
        let mut c = Collections::default();
        let me = Did::from("did:plc:me");
        let post = AtUri::from("at://a/p/1");
        assert_eq!(c.create(&me, "  ", None), None);
        let a = c.create(&me, "Recipes", Some(post.clone())).unwrap();
        let b = c.create(&me, "Art", None).unwrap();
        assert_ne!(a, b);
        assert_eq!(c.list("did:plc:me")[0].name, "Art", "newest first");
        assert!(c.list("did:plc:other").is_empty());
        assert!(c.toggle("did:plc:me", b, &post));
        c.rename("did:plc:me", a, "Food");
        c.delete("did:plc:me", b);
        let back: Collections = serde_json::from_slice(&serde_json::to_vec(&c).unwrap()).unwrap();
        let list = back.list("did:plc:me");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Food");
        assert_eq!(list[0].posts, [post]);
        c.forget_account("did:plc:me");
        assert!(c.list("did:plc:me").is_empty());
    }
}
