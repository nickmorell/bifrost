//! Your own collections of posts, kept only on this computer: folders for
//! bookmarks that nobody else sees.

use super::AtUri;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    /// Milliseconds since the epoch when it was made; unique per account.
    pub id: u64,
    pub name: String,
    /// The posts in it, newest added first.
    #[serde(default)]
    pub posts: Vec<AtUri>,
}

impl Collection {
    pub fn has(&self, uri: &AtUri) -> bool {
        self.posts.contains(uri)
    }

    /// Adds the post, or takes it out if it is in already. Says whether it is in now.
    pub fn toggle(&mut self, uri: &AtUri) -> bool {
        if let Some(i) = self.posts.iter().position(|p| p == uri) {
            self.posts.remove(i);
            false
        } else {
            self.posts.insert(0, uri.clone());
            true
        }
    }
}

/// A name for a collection: trimmed, at most 64 characters, never empty.
pub fn collection_name(name: &str) -> Option<String> {
    let name: String = name.trim().chars().take(64).collect();
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posts_go_in_and_out_newest_first() {
        let mut c = Collection {
            id: 1,
            name: "Recipes".into(),
            posts: Vec::new(),
        };
        let (a, b) = (AtUri::from("at://a/p/1"), AtUri::from("at://a/p/2"));
        assert!(c.toggle(&a));
        assert!(c.toggle(&b));
        assert_eq!(c.posts, [b.clone(), a.clone()]);
        assert!(!c.toggle(&a));
        assert!(!c.has(&a) && c.has(&b));
        assert_eq!(collection_name("  Cats  ").as_deref(), Some("Cats"));
        assert_eq!(collection_name("   "), None);
        assert_eq!(collection_name(&"x".repeat(100)).unwrap().len(), 64);
    }
}
