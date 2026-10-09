//! The tabs under a profile header.

use crate::api::AuthorFilter;

/// The tabs under a profile header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Posts,
    Replies,
    Media,
    Likes,
    Feeds,
    Lists,
    StarterPacks,
}

impl Tab {
    /// The timeline the tab shows, if it is one.
    pub fn filter(self) -> Option<AuthorFilter> {
        match self {
            Tab::Posts => Some(AuthorFilter::Posts),
            Tab::Replies => Some(AuthorFilter::Replies),
            Tab::Media => Some(AuthorFilter::Media),
            Tab::Likes => Some(AuthorFilter::Likes),
            Tab::Feeds | Tab::Lists | Tab::StarterPacks => None,
        }
    }
}
