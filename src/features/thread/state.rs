//! The threads that have been opened, as loaded or with why they failed.

use super::tree;
use crate::components::Heights;
use crate::model::{AtUri, PostView, ThreadNode};
use std::collections::HashMap;

/// Every thread opened, by the post it was opened on. A failure keeps its wording
/// for people until it is tried again.
#[derive(Debug, Default)]
pub struct Threads {
    pub(super) loaded: HashMap<AtUri, Result<ThreadNode, String>>,
    /// How tall each row was last drawn, so only the rows near the scroll position
    /// are drawn.
    pub(super) heights: Heights,
}

impl Threads {
    /// Whether the thread has been fetched, whether it worked or not.
    pub fn contains(&self, uri: &str) -> bool {
        self.loaded.contains_key(uri)
    }

    /// Drops the thread if it failed, so opening it again tries again.
    pub fn forget_failed(&mut self, uri: &str) {
        if matches!(self.loaded.get(uri), Some(Err(_))) {
            self.loaded.remove(uri);
        }
    }

    pub fn forget(&mut self, uri: &str) {
        self.loaded.remove(uri);
    }

    /// Takes a deleted post out of every loaded thread. Its replies move up to the
    /// post it answered, so the rest of a thread can still be read and deleted; a
    /// thread opened on it is forgotten.
    pub fn remove_post(&mut self, uri: &str) {
        self.loaded.remove(uri);
        for node in self.loaded.values_mut().filter_map(|t| t.as_mut().ok()) {
            tree::remove(node, uri);
        }
    }

    pub fn clear(&mut self) {
        self.loaded.clear();
        self.heights.clear();
    }

    /// Once more than `max` threads are kept, forgets those `keep` does not want.
    pub fn trim(&mut self, max: usize, keep: impl Fn(&AtUri) -> bool) {
        if self.loaded.len() > max {
            self.loaded.retain(|uri, _| keep(uri));
        }
    }

    /// The posts of one loaded thread: the post, the posts above it, then its replies.
    pub fn posts_of(&self, uri: &str) -> Vec<&PostView> {
        let mut out = Vec::new();
        if let Some(Ok(node)) = self.loaded.get(uri) {
            tree::collect(node, &mut out);
        }
        out
    }

    /// The posts of every loaded thread.
    pub fn posts(&self) -> Vec<&PostView> {
        let mut out = Vec::new();
        for node in self.loaded.values().filter_map(|t| t.as_ref().ok()) {
            tree::collect(node, &mut out);
        }
        out
    }

    /// Applies `f` to every post in every loaded thread.
    pub fn each_post_mut(&mut self, f: &mut dyn FnMut(&mut PostView)) {
        for node in self.loaded.values_mut().filter_map(|t| t.as_mut().ok()) {
            tree::walk_mut(node, f);
        }
    }

    /// The post at `uri`, if a loaded thread has it.
    pub fn find_post(&self, uri: &str) -> Option<&PostView> {
        self.loaded
            .values()
            .filter_map(|t| t.as_ref().ok())
            .find_map(|node| tree::find(node, uri))
    }
}
