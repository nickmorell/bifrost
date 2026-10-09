//! The timelines kept for the screens and deck columns that show them.

use super::key::Key;
use crate::components::Heights;
use crate::model::{FeedItem, FeedReplyContext, MaybePost, PostView};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug, Default)]
pub struct Timeline {
    pub(super) items: Vec<FeedItem>,
    pub(super) cursor: Option<String>,
    pub(super) loading: bool,
    /// The server returned no cursor: there is nothing more to load.
    pub(super) done: bool,
    pub(super) error: Option<String>,
    /// Newer posts found by a background check, waiting for "Show new posts".
    pub(super) pending: Vec<FeedItem>,
    /// The check found no overlap with what is shown: showing the new posts replaces
    /// the list instead of adding to its top.
    pub(super) pending_gap: Option<Gap>,
    /// How tall each post was last drawn, by `item_key`, so only the posts near the
    /// scroll position need drawing.
    pub(super) heights: Heights,
    /// Where each loaded page ended: its length and the cursor that continues it.
    pub(super) pages: Vec<(usize, Option<String>)>,
    /// The entry you were reading when BiFrost last showed this timeline, while it is
    /// being looked for.
    pub(super) seek: Option<String>,
    /// That entry, once found below newer ones: "You left off here" goes above it.
    pub(super) marker: Option<String>,
}

impl Timeline {
    pub fn items(&self) -> &[FeedItem] {
        &self.items
    }

    /// How many newer posts a background check found.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// The newer posts found don't reach the ones shown, so there may be more.
    pub fn pending_gap(&self) -> bool {
        self.pending_gap.is_some()
    }
}

/// Where a timeline carries on after newer posts that don't reach the ones shown.
#[derive(Debug)]
pub(super) struct Gap {
    pub(super) cursor: Option<String>,
}

/// Posts taken out of timelines, with where they were, to put them back: each
/// timeline item as it was, and whether it was taken out (`true`) or only lost the
/// deleted post shown above it as the post it answers (`false`).
pub type Removed = Vec<(Key, usize, FeedItem, bool)>;

/// Every timeline loaded, by what it shows.
#[derive(Debug, Default)]
pub struct Timelines {
    pub(super) by_key: HashMap<Key, Timeline>,
}

impl Timelines {
    pub fn get(&self, key: &Key) -> Option<&Timeline> {
        self.by_key.get(key)
    }

    pub fn contains(&self, key: &Key) -> bool {
        self.by_key.contains_key(key)
    }

    pub fn count(&self) -> usize {
        self.by_key.len()
    }

    pub fn clear(&mut self) {
        self.by_key.clear();
    }

    /// Forgets a timeline; it loads again when next shown.
    pub fn forget(&mut self, key: &Key) {
        self.by_key.remove(key);
    }

    /// Keeps only the timelines `keep` wants.
    pub fn retain(&mut self, mut keep: impl FnMut(&Key) -> bool) {
        self.by_key.retain(|k, _| keep(k));
    }

    /// Cuts every timeline longer than `max` posts back to its first pages, unless
    /// `keep` says it is on screen.
    pub fn shorten(&mut self, max: usize, keep: impl Fn(&Key) -> bool) {
        for (key, tl) in &mut self.by_key {
            if tl.items.len() > max && !keep(key) {
                tl.keep_pages(max);
            }
        }
    }

    /// Drops the posts of `key` that `keep` turns down.
    pub fn filter(&mut self, key: &Key, keep: impl FnMut(&FeedItem) -> bool) {
        if let Some(tl) = self.by_key.get_mut(key) {
            tl.items.retain(keep);
        }
    }

    /// After a failed "load more": forgets the failure and says whether the timeline
    /// has posts, so the next page can be asked for again.
    pub fn retry_more(&mut self, key: &Key) -> bool {
        self.by_key
            .get_mut(key)
            .is_some_and(|tl| tl.error.take().is_some() && !tl.items.is_empty())
    }

    /// Puts the newer posts a background check found on top, and returns where they
    /// went, or `None` if there were none.
    pub fn show_pending(&mut self, key: &Key) -> Option<Range<usize>> {
        let tl = self.by_key.entry(key.clone()).or_default();
        if tl.pending.is_empty() {
            return None;
        }
        let fresh = std::mem::take(&mut tl.pending);
        let n = fresh.len();
        if let Some(gap) = tl.pending_gap.take() {
            tl.items = fresh;
            tl.cursor = gap.cursor;
            tl.done = tl.cursor.is_none();
            tl.pages = vec![(tl.items.len(), tl.cursor.clone())];
            tl.heights.clear();
        } else {
            tl.items.splice(0..0, fresh);
            for (len, _) in &mut tl.pages {
                *len += n;
            }
        }
        Some(0..n)
    }

    /// Takes a deleted post out of every timeline.
    pub fn remove_post(&mut self, uri: &str) -> Removed {
        let mut removed = Vec::new();
        for (key, tl) in &mut self.by_key {
            let mut i = 0;
            tl.items.retain_mut(|item| {
                if item.post.uri == uri {
                    removed.push((key.clone(), i, item.clone(), true));
                    i += 1;
                    return false;
                }
                let answers_it = matches!(
                    &item.reply,
                    Some(FeedReplyContext { parent: MaybePost::Post(p) }) if p.uri == uri
                );
                if answers_it {
                    removed.push((key.clone(), i, item.clone(), false));
                    item.reply = Some(FeedReplyContext {
                        parent: MaybePost::Unavailable,
                    });
                }
                i += 1;
                true
            });
        }
        removed
    }

    /// Puts posts taken out by [`Timelines::remove_post`] back where they were.
    pub fn restore(&mut self, removed: Removed) {
        for (key, i, item, taken) in removed {
            if let Some(tl) = self.by_key.get_mut(&key) {
                if taken {
                    let i = i.min(tl.items.len());
                    tl.items.insert(i, item);
                } else if let Some(changed) =
                    tl.items.iter_mut().find(|t| t.post.uri == item.post.uri)
                {
                    *changed = item;
                }
            }
        }
    }

    /// Every post in every timeline.
    pub fn posts(&self) -> impl Iterator<Item = &PostView> {
        self.by_key
            .values()
            .flat_map(|tl| tl.items.iter().map(|i| &i.post))
    }

    pub fn posts_mut(&mut self) -> impl Iterator<Item = &mut PostView> {
        self.by_key
            .values_mut()
            .flat_map(|tl| tl.items.iter_mut().map(|i| &mut i.post))
    }

    /// Applies `apply` to every post in every timeline, and to the posts they answer.
    pub fn each_post_mut(&mut self, apply: &mut impl FnMut(&mut PostView)) {
        for item in self.by_key.values_mut().flat_map(|tl| &mut tl.items) {
            apply(&mut item.post);
            if let Some(FeedReplyContext {
                parent: MaybePost::Post(p),
            }) = &mut item.reply
            {
                apply(p);
            }
        }
    }
}
