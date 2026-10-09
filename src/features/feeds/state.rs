//! The Home tabs, and what is known about your feeds.

use crate::model::{FeedGenerator, FeedSource};
use std::collections::HashMap;

/// A pinned feed as a tab on Home.
#[derive(Debug, Clone)]
pub struct Tab {
    pub source: FeedSource,
    pub name: String,
}

/// The name a tab shows until its feed's name is known.
pub(super) const UNNAMED: &str = "…";

#[derive(Debug, Default)]
pub struct Feeds {
    pub(super) tabs: Vec<Tab>,
    pub(super) active: usize,
    /// Names and pictures of saved feeds, including ones that are not pinned.
    pub(super) info: HashMap<FeedSource, (String, Option<String>)>,
    /// Feeds Bluesky suggests.
    pub(super) suggested: Option<Result<Vec<FeedGenerator>, String>>,
}

impl Feeds {
    /// The Home tabs, in order.
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// The Home tab that is open.
    pub fn active(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    /// Whether `source` is one of the Home tabs.
    pub fn is_tab(&self, source: &FeedSource) -> bool {
        self.tabs.iter().any(|t| &t.source == source)
    }

    /// Just Following, until your pinned feeds are known.
    pub fn following_only(&mut self) {
        self.tabs = vec![Tab {
            source: FeedSource::Following,
            name: "Following".into(),
        }];
        self.active = 0;
    }

    /// The name of a saved feed, once looked up.
    pub fn name(&self, source: &FeedSource) -> Option<&str> {
        self.info.get(source).map(|(n, _)| n.as_str())
    }

    /// Bluesky's suggestions, or none while they load or after they failed.
    pub fn suggested(&self) -> &[FeedGenerator] {
        match &self.suggested {
            Some(Ok(feeds)) => feeds,
            _ => &[],
        }
    }

    /// Forgets what was looked up, for another account.
    pub fn forget(&mut self) {
        self.info.clear();
        self.suggested = None;
    }

    /// Forgets names of feeds that are not tabs once more than `max` have piled up.
    pub fn trim(&mut self, max: usize) {
        if self.info.len() > max {
            let tabs = &self.tabs;
            self.info
                .retain(|source, _| tabs.iter().any(|t| &t.source == source));
        }
    }
}
