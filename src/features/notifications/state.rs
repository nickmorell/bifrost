//! The notifications loaded so far and how many are unseen.

use super::Filter;
use super::group::{Group, group};
use crate::components::Heights;
use crate::model::{ActorViewer, AtUri, Notification, PostView};
use std::collections::{HashMap, HashSet};

/// The notifications list for the signed-in account.
#[derive(Debug, Default)]
pub struct Notifications {
    pub(super) items: Vec<Notification>,
    /// Posts the notifications refer to, by URI.
    pub(super) subjects: HashMap<AtUri, PostView>,
    pub(super) cursor: Option<String>,
    pub(super) loading: bool,
    pub(super) loaded: bool,
    pub(super) filter: Filter,
    /// Grouped rows opened to list everyone in them, by their newest notification's URI.
    pub(super) expanded: HashSet<String>,
    /// Arrived since they were last seen.
    pub(super) unread: u64,
    /// The rows the list shows, grouped once when the notifications or the filter
    /// change rather than on every frame.
    pub(super) rows: Vec<Group>,
    /// How tall each row was last drawn, so only the rows near the scroll position
    /// are drawn.
    pub(super) heights: Heights,
}

impl Notifications {
    pub fn unread(&self) -> u64 {
        self.unread
    }

    /// Whether opening the list should reload it.
    pub fn is_stale(&self) -> bool {
        !self.loaded || self.unread > 0
    }

    /// Whether an older page can be loaded now.
    pub fn has_more(&self) -> bool {
        self.cursor.is_some() && !self.loading
    }

    /// Once more than `max` are loaded, starts over from the newest next time the list
    /// opens, keeping the filter and the unseen count.
    pub fn trim(&mut self, max: usize) {
        if self.items.len() > max {
            *self = Self {
                filter: self.filter,
                unread: self.unread,
                ..Self::default()
            };
        }
    }

    /// Groups the notifications into rows again after they or the filter changed.
    pub(super) fn regroup(&mut self) {
        self.rows = group(&self.items, self.filter);
    }

    /// The post a notification is about, if it was loaded.
    pub fn post(&self, uri: &str) -> Option<&PostView> {
        self.subjects.get(uri)
    }

    /// Every post the notifications are about.
    pub fn posts(&self) -> impl Iterator<Item = &PostView> {
        self.subjects.values()
    }

    pub fn posts_mut(&mut self) -> impl Iterator<Item = &mut PostView> {
        self.subjects.values_mut()
    }

    /// Applies `f` to how you relate to `did` wherever they notified you.
    pub fn with_author(&mut self, did: &str, f: &mut dyn FnMut(&mut ActorViewer)) {
        for n in &mut self.items {
            if n.author.did == did {
                f(&mut n.author.viewer);
            }
        }
    }
}
