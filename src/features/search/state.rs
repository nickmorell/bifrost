//! What the search screen keeps: the query, results per tab, suggestions while typing
//! and the Explore page.

use super::tab::Tab;
use crate::model::{Author, FeedGenerator, PostView, Trend};

/// The id of the search box, so the `/` shortcut and "Clear" can focus it.
pub const INPUT: &str = "search-input";

#[derive(Debug, Default)]
pub struct Search {
    pub(super) query: String,
    pub(super) submitted: String,
    pub(super) tab: Option<Tab>,
    pub(super) posts: Vec<PostView>,
    /// Which of Top or Latest `posts` holds.
    pub(super) posts_sort: Option<Tab>,
    pub(super) posts_cursor: Option<String>,
    pub(super) people: Vec<Author>,
    pub(super) people_cursor: Option<String>,
    pub(super) feeds: Vec<FeedGenerator>,
    pub(super) feeds_cursor: Option<String>,
    pub(super) posts_loading: bool,
    pub(super) people_loading: bool,
    pub(super) feeds_loading: bool,
    /// The query has changed since it was last searched: show suggestions, not results.
    pub(super) typing: bool,
    /// People matching what is being typed.
    pub(super) suggest: Vec<Author>,
    pub(super) suggest_seq: u64,
    pub(super) trends: Option<Result<Vec<Trend>, String>>,
    pub(super) suggested_people: Vec<Author>,
    /// Bumped on every new query; responses tagged with an older value are dropped.
    pub(super) seq: u64,
}

impl Search {
    /// Whether a search has been run, so results show instead of the Explore page.
    pub fn searched(&self) -> bool {
        !self.submitted.is_empty()
    }

    /// The `#tag` this search is for, when it is a single hashtag.
    pub fn tag(&self) -> Option<&str> {
        let t = self.submitted.strip_prefix('#')?;
        (!t.is_empty() && !t.contains(char::is_whitespace)).then_some(t)
    }

    /// The posts found.
    pub fn posts(&self) -> &[PostView] {
        &self.posts
    }

    pub fn posts_mut(&mut self) -> impl Iterator<Item = &mut PostView> {
        self.posts.iter_mut()
    }

    pub fn post(&self, uri: &str) -> Option<&PostView> {
        self.posts.iter().find(|p| p.uri == uri)
    }

    /// The people found.
    pub fn people_mut(&mut self) -> impl Iterator<Item = &mut Author> {
        self.people.iter_mut()
    }

    pub(super) fn loading(&self, tab: Tab) -> bool {
        match tab {
            Tab::Top | Tab::Latest => self.posts_loading,
            Tab::People => self.people_loading,
            Tab::Feeds => self.feeds_loading,
            Tab::Saved => false,
        }
    }
}
