//! What you do to posts and accounts: likes, reposts, bookmarks, deleting, pinning,
//! muting threads, following, muting and blocking, and who can reply to your posts
//! ([`who`]).
//!
//! Each change shows at once and is put back if the server refuses it. The posts and
//! accounts themselves are cached all over the app, so this feature doesn't change
//! them: it reports each change as an [`Event`] carrying a [`PostChange`] or
//! [`ActorChange`] the app applies to every copy.

mod change;
mod message;
mod update;
pub mod who;

#[cfg(test)]
mod tests;

pub use change::{ActorChange, PostChange};
pub use message::{Event, Known, Msg, Toggle};

use crate::model::AtUri;

/// What the actions keep between messages.
#[derive(Debug, Default)]
pub struct Actions {
    /// The post whose Delete was pressed once and waits to be confirmed.
    confirm_delete: Option<AtUri>,
}

impl Actions {
    /// The post waiting for its delete to be confirmed.
    pub fn confirm_delete(&self) -> Option<&AtUri> {
        self.confirm_delete.as_ref()
    }

    /// Forgets a delete waiting to be confirmed.
    pub fn cancel_delete(&mut self) {
        self.confirm_delete = None;
    }
}
