//! Typed identifiers, so a DID can't be passed where a record URI is expected (or the
//! other way round). Both serialize as plain strings and read as `&str` where a string
//! is needed.

use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

macro_rules! id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;
            fn deref(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> String {
                id.0
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool {
                &self.0 == other
            }
        }

        impl PartialEq<$name> for str {
            fn eq(&self, other: &$name) -> bool {
                self == other.0
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0
            }
        }

        impl PartialEq<$name> for String {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0
            }
        }
    };
}

id!(
    /// An account's permanent id, such as `did:plc:abc123`.
    Did
);

id!(
    /// A record's address, `at://<did>/<collection>/<rkey>`: a post, list, feed, like,
    /// follow and so on.
    AtUri
);

impl AtUri {
    /// The address of record `rkey` in `collection` of `did`'s repository.
    pub fn new(did: &str, collection: &str, rkey: &str) -> Self {
        Self(format!("at://{did}/{collection}/{rkey}"))
    }

    /// The account the record belongs to.
    pub fn did(&self) -> Option<&str> {
        self.0.strip_prefix("at://")?.split('/').next()
    }

    /// The record's collection, such as `app.bsky.feed.post`.
    pub fn collection(&self) -> Option<&str> {
        self.0.strip_prefix("at://")?.split('/').nth(1)
    }

    /// The record's key, last in the address.
    pub fn rkey(&self) -> Option<&str> {
        self.0.strip_prefix("at://")?.split('/').nth(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_read_and_compare_as_strings() {
        let did = Did::from("did:plc:a");
        assert_eq!(did, "did:plc:a");
        assert_eq!(did.len(), 9);
        let json = serde_json::to_string(&did).unwrap();
        assert_eq!(json, "\"did:plc:a\"");
        assert_eq!(serde_json::from_str::<Did>(&json).unwrap(), did);
    }

    #[test]
    fn at_uri_parts() {
        let uri = AtUri::new("did:plc:a", "app.bsky.feed.post", "1");
        assert_eq!(uri, "at://did:plc:a/app.bsky.feed.post/1");
        assert_eq!(uri.did(), Some("did:plc:a"));
        assert_eq!(uri.rkey(), Some("1"));
        assert_eq!(uri.collection(), Some("app.bsky.feed.post"));
        assert_eq!(AtUri::from("https://x").did(), None);
    }
}
