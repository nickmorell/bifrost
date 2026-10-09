//! Reading position: which post sits at the top of a scrolled timeline, where a post
//! sits, and finding the post you were reading when a timeline loads again.

use super::key::Key;
use super::merge::item_key;
use super::message::Event;
use super::state::{Timeline, Timelines};
use crate::components::row_key;

/// Height assumed for a post not drawn yet.
pub(super) const ESTIMATED_POST: f32 = 240.0;
/// Pages loaded while looking for the post you were reading, before giving up.
const MAX_PAGES: usize = 5;

impl Timeline {
    /// How tall the entry `key` was last drawn, or a guess.
    pub(super) fn height(&self, key: &str) -> f32 {
        self.heights.get(row_key(key)).unwrap_or(ESTIMATED_POST)
    }

    /// Where the entry `key` starts, from the top of the timeline.
    pub fn top_of(&self, key: &str) -> Option<f32> {
        let mut top = 0.0;
        for item in &self.items {
            let k = item_key(item);
            if k == key {
                return Some(top);
            }
            top += self.height(&k);
        }
        None
    }

    /// The entry at the top of the view when the timeline is scrolled `offset` down.
    pub fn item_at(&self, offset: f32) -> Option<String> {
        let mut bottom = 0.0;
        for item in &self.items {
            let k = item_key(item);
            bottom += self.height(&k);
            if bottom > offset {
                return Some(k);
            }
        }
        None
    }

    /// The entry marked as where you left off.
    pub fn marker(&self) -> Option<&str> {
        self.marker.as_deref()
    }

    /// Looks for the entry being sought among those loaded. It is found (`Some(true)`),
    /// worth another page (`Some(false)`), or given up on (`None`).
    fn look(&mut self) -> Option<bool> {
        let want = self.seek.as_ref()?;
        if let Some(i) = self.items.iter().position(|it| item_key(it) == *want) {
            // Nothing newer came in above it: no need to mark it.
            self.marker = (i > 0).then(|| want.clone());
            return Some(true);
        }
        (!self.done && self.cursor.is_some() && self.pages.len() < MAX_PAGES).then_some(false)
    }
}

/// What looking for the post you were reading came to.
pub(super) enum Look {
    Found(Event),
    MorePlease,
    Nothing,
}

impl Timelines {
    /// Starts looking for the entry `item` in `key`, which is loading or about to.
    pub(super) fn seek(&mut self, key: &Key, item: String) -> Look {
        let tl = self.by_key.entry(key.clone()).or_default();
        tl.seek = Some(item);
        if tl.loading || tl.items.is_empty() {
            return Look::Nothing;
        }
        self.look(key)
    }

    /// After a page of `key` arrived: whether the entry sought is in it.
    pub(super) fn look(&mut self, key: &Key) -> Look {
        let Some(tl) = self.by_key.get_mut(key) else {
            return Look::Nothing;
        };
        match tl.look() {
            Some(true) => {
                let item = tl.seek.take().unwrap_or_default();
                Look::Found(Event::Found(key.clone(), item))
            }
            Some(false) => Look::MorePlease,
            None => {
                tl.seek = None;
                Look::Nothing
            }
        }
    }
}
