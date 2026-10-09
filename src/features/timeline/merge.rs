//! Fitting a newly fetched page in with the posts already shown, and cutting a long
//! timeline back to its first pages.

use super::state::Timeline;
use crate::components::row_key;
use crate::model::{FeedItem, FeedReason};
use std::collections::HashSet;

/// A timeline entry's identity: its post, and who reposted it if it is a repost.
pub fn item_key(item: &FeedItem) -> String {
    match &item.reason {
        Some(FeedReason::Repost { by }) => format!("{}#{}", item.post.uri, by.did),
        _ => item.post.uri.to_string(),
    }
}

/// Where the newest page meets the posts already shown.
pub(super) enum Merge {
    /// These posts are new and go on top; the rest of the page was already shown.
    Prepend(Vec<FeedItem>),
    /// Nothing on the page was shown before, so there may be a gap below it.
    Gap(Vec<FeedItem>),
}

pub(super) fn merge_newest(shown: &[FeedItem], page: Vec<FeedItem>) -> Merge {
    let seen: HashSet<String> = shown.iter().map(item_key).collect();
    match page.iter().position(|i| seen.contains(&item_key(i))) {
        Some(at) => {
            let mut page = page;
            page.truncate(at);
            Merge::Prepend(page)
        }
        None if shown.is_empty() => Merge::Prepend(page),
        None => Merge::Gap(page),
    }
}

impl Timeline {
    /// Drops the pages after the last one that ends within `max` posts (always keeping
    /// the first), so loading more carries on from there.
    pub(super) fn keep_pages(&mut self, max: usize) {
        let Some(at) = self
            .pages
            .iter()
            .rposition(|(len, _)| *len <= max)
            .or((!self.pages.is_empty()).then_some(0))
        else {
            return;
        };
        let (len, cursor) = self.pages[at].clone();
        if len >= self.items.len() {
            return;
        }
        self.items.truncate(len);
        self.pages.truncate(at + 1);
        self.cursor = cursor;
        self.done = self.cursor.is_none();
        let kept: HashSet<u64> = self
            .items
            .iter()
            .map(|i| row_key(item_key(i).as_str()))
            .collect();
        self.heights.retain(|k| kept.contains(&k));
    }
}
