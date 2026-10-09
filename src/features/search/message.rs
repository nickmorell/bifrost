//! What the search screen sends and asks for.

use super::tab::Tab;
use crate::api::ApiError;
use crate::model::{AtUri, Author, FeedGenerator, FeedSource, Page, PostView, Trend};

/// A message for search.
#[derive(Debug, Clone)]
pub enum Msg {
    /// The search box changed.
    Input(String),
    /// Typing paused: suggest people, if this is still the latest input.
    Suggest(u64),
    Suggested(u64, Result<Vec<Author>, ApiError>),
    /// Search for what is in the box.
    Submit,
    /// Search for this now, as if typed and submitted.
    For(String),
    /// Empty the box and go back to the Explore page.
    Clear,
    /// Remove one recent search, or all of them.
    Forget(Option<String>),
    /// A result tab was picked.
    Tab(Tab),
    /// The results were scrolled to the end: load the next page of the open tab.
    More,
    Posts(u64, Tab, bool, Result<Page<PostView>, ApiError>),
    People(u64, bool, Result<Page<Author>, ApiError>),
    Feeds(u64, bool, Result<Page<FeedGenerator>, ApiError>),

    /// Load what the Explore page shows that is missing; `force` loads it all again.
    Explore {
        force: bool,
    },
    Trends(Result<Vec<Trend>, ApiError>),
    SuggestedPeople(Result<Vec<Author>, ApiError>),
    OpenTrend(Trend),
    /// The feed a trend points to was found (by its name), or not.
    TrendFeed(String, Result<AtUri, ApiError>),
    /// "More feeds" on the Explore page.
    MoreFeeds,

    /// "Post with tag" on a #tag page.
    PostWithTag(String),
    /// Mute (`true`) or unmute a tag, by the muted word's text.
    MuteTag(String, bool),
}

/// What the shell has to do for search.
#[derive(Debug, Clone)]
pub enum Event {
    /// Load these pictures.
    Pictures(Vec<String>),
    /// These posts were found: load their pictures.
    Posts(Vec<AtUri>),
    /// A search failed: say so, and check whether the session ended.
    Failed(ApiError),
    /// Open the search screen, if it is not open.
    Show,
    /// Remember this query among recent searches.
    Remember(String),
    /// Forget one recent search, or all of them.
    Forget(Option<String>),
    /// Load what the Explore page shows, all of it again when `true`.
    Explore(bool),
    OpenFeed(FeedSource, String),
    /// Open the Feeds screen.
    MoreFeeds,
    PostWithTag(String),
    MuteTag(String, bool),
}
