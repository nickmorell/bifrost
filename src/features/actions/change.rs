//! Changes to the posts and accounts the app has cached.

use crate::model::{ActorViewer, Mine, PostView};

/// A change to every copy of one post.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostChange {
    /// Your like is now `mine`; the count moves by one up (`true`) or down.
    Like(Option<Mine>, Option<bool>),
    Repost(Option<Mine>, Option<bool>),
    Bookmarked(bool),
    /// The post is in a thread you muted (`true`) or unmuted.
    ThreadMuted(bool),
}

impl PostChange {
    pub fn apply(&self, p: &mut PostView) {
        let step = |count: &mut u64, up: Option<bool>| match up {
            Some(true) => *count += 1,
            Some(false) => *count = count.saturating_sub(1),
            None => {}
        };
        match self {
            PostChange::Like(mine, up) => {
                p.viewer.like.clone_from(mine);
                step(&mut p.like_count, *up);
            }
            PostChange::Repost(mine, up) => {
                p.viewer.repost.clone_from(mine);
                step(&mut p.repost_count, *up);
            }
            PostChange::Bookmarked(on) => p.viewer.bookmarked = *on,
            PostChange::ThreadMuted(on) => p.viewer.thread_muted = *on,
        }
    }
}

/// A change to how you relate to one account, on every copy of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorChange {
    Following(Option<Mine>),
    Blocking(Option<Mine>),
    Muted(bool),
}

impl ActorChange {
    pub fn apply(&self, v: &mut ActorViewer) {
        match self {
            ActorChange::Following(mine) => v.following.clone_from(mine),
            ActorChange::Blocking(mine) => v.blocking.clone_from(mine),
            ActorChange::Muted(on) => v.muted = *on,
        }
    }
}
