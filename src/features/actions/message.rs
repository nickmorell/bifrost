//! What the actions are told, what they need to know, and what they report back.

use super::change::{ActorChange, PostChange};
use crate::api::ApiError;
use crate::model::{ActorViewer, AtUri, Did, PostView, StrongRef};

/// Turning something on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    On,
    Off,
}

impl Toggle {
    pub fn on(self) -> bool {
        self == Toggle::On
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Like(AtUri, Toggle),
    Repost(AtUri, Toggle),
    Bookmark(AtUri, Toggle),
    /// Mute or unmute the thread whose first post is at this URI.
    MuteThread(AtUri, Toggle),
    /// Pin a post to the top of your profile, or unpin it.
    Pin(AtUri, Toggle),
    /// Delete was pressed once: ask again.
    AskDelete(AtUri),
    /// Delete was confirmed.
    Delete(AtUri),
    Follow(Did, Toggle),
    Mute(Did, Toggle),
    Block(Did, Toggle),
    /// A like, repost, follow or block was saved at a URI, or refused.
    Created(Made, Result<AtUri, ApiError>),
    /// A write finished; on failure its change is put back.
    Done(Undo, Result<(), ApiError>),
}

/// A record being created.
#[derive(Debug, Clone)]
pub enum Made {
    Like(AtUri),
    Repost(AtUri),
    Follow(Did),
    Block(Did),
}

/// How to put back a change the server refused.
#[derive(Debug, Clone)]
pub enum Undo {
    Post(AtUri, PostChange, &'static str),
    Thread(AtUri, PostChange, &'static str),
    Actor(Did, ActorChange, &'static str),
    /// Put a deleted post back.
    Delete(AtUri),
    /// Show this as the pinned post again.
    Pin(Option<StrongRef>),
}

/// What the actions need to know about what the app has cached.
pub trait Known {
    fn post(&self, uri: &AtUri) -> Option<PostView>;
    fn actor(&self, did: &Did) -> Option<ActorViewer>;
    /// The post pinned to your profile.
    fn pinned(&self) -> Option<StrongRef>;
}

/// What the app has to do.
#[derive(Debug)]
pub enum Event {
    /// Change every copy of a post.
    Post(AtUri, PostChange),
    /// Change every post in the thread whose first post is at this URI.
    Thread(AtUri, PostChange),
    /// Change every copy of an account.
    Actor(Did, ActorChange),
    /// Take a post out of everywhere it is shown: it is being deleted.
    Remove(AtUri),
    /// The delete went through: what was kept to put the post back can go.
    Removed(AtUri),
    /// The delete was refused: put the post back.
    PutBack(AtUri),
    /// Show this post as the one pinned to your profile.
    ShowPinned(Option<StrongRef>),
    /// The pinned post was saved: your posts list is fetched again.
    PinSaved,
    Toast(String),
    /// `action` failed; say so, unless the session ended.
    Failed(&'static str, ApiError),
}
