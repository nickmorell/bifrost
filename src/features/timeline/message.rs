//! What timelines are told, and what they report back.

use super::key::{Key, Load};
use crate::api::ApiError;
use crate::model::{FeedFilter, FeedItem, FollowingPrefs, Page};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum Msg {
    /// Loads the first page of a timeline, unless it is loaded. With `force`, a timeline
    /// already shown gets its newest posts on top instead.
    Load {
        key: Key,
        force: bool,
    },
    /// Loads the next page, unless one is loading or there are no more.
    More(Key),
    /// Checks a shown timeline for newer posts, without showing them yet.
    Peek(Key),
    /// Puts the newest posts on top, loading the timeline if it is not.
    Refresh(Key),
    Loaded(Key, Load, Result<Page<FeedItem>, ApiError>),
    /// A post was drawn this tall.
    Measured(Key, u64, f32),
    /// Look for the entry you were reading (by `item_key`) as the timeline loads,
    /// loading a few more pages if need be.
    Find(Key, String),
}

/// What loading a page needs from the app.
pub struct Ctx<'a> {
    /// What to leave out of the Following timeline.
    pub following: &'a FollowingPrefs,
    /// The account signed in, whose own posts Following always keeps.
    pub me: &'a str,
    /// What each feed leaves out, by [`crate::model::FeedSource::id`].
    pub filters: &'a BTreeMap<String, FeedFilter>,
}

/// What the app has to do after a timeline changed.
#[derive(Debug)]
pub enum Event {
    /// A page arrived and is shown: fetch the pictures of its posts.
    Fetched(Vec<FeedItem>),
    /// So little of what was loaded of `key` is left after its filters that there may
    /// be nothing to scroll: load its next page if it is on screen.
    Empty(Key),
    /// The entry you were reading was found: scroll to it.
    Found(Key, String),
    /// A page failed to load; the timeline shows why, so only an ended session matters.
    Failed(ApiError),
}
