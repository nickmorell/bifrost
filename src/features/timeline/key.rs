//! Which timeline, and why a page of it is fetched.

use crate::api::AuthorFilter;
use crate::model::FeedSource;

/// What a timeline shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Feed(FeedSource),
    Author(String, AuthorFilter),
    Bookmarks,
}

/// Why a page of a timeline was fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Load {
    /// The first page of a timeline not shown before.
    First,
    /// The next page, at the bottom.
    More,
    /// The newest page, merged on top of what is loaded.
    Refresh,
    /// The newest page, checked in the background and held until asked for.
    Peek,
}
