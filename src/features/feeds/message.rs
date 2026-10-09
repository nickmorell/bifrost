//! What your feeds are told, and what they ask of the app.

use crate::api::ApiError;
use crate::model::{FeedGenerator, FeedSource, PrefEdit};

#[derive(Debug, Clone)]
pub enum Msg {
    /// The saved feeds changed: rebuild the Home tabs, keeping the open one.
    Sync,
    /// Your preferences were loaded: build the Home tabs from scratch.
    Reset,
    /// Tab names looked up, by source; `None` means the feed no longer exists.
    Names(Vec<(FeedSource, Option<String>)>),
    /// The Feeds screen is shown: look up the names of saved feeds and Bluesky's
    /// suggestions, all again when `force`.
    Load {
        force: bool,
    },
    /// Only Bluesky's suggestions, for the Explore page; all again when `force`.
    Suggest {
        force: bool,
    },
    /// Names and pictures of saved feeds.
    Info(Vec<(FeedSource, String, Option<String>)>),
    Suggested(Result<Vec<FeedGenerator>, ApiError>),
    /// A Home tab was clicked.
    Select(usize),
    /// Save, pin, move or remove a feed.
    Edit(PrefEdit),
    Open(FeedSource, String),
    Home,
    /// The button to pin more feeds.
    Discover,
    Retry,
    SignInAgain,
}

/// What the app has to do for your feeds.
#[derive(Debug)]
pub enum Event {
    /// Load these feed pictures.
    Pictures(Vec<String>),
    /// Change your preferences, and save them.
    Edit(PrefEdit),
    /// Open the feed or list with this name.
    Open(FeedSource, String),
    /// Show Home.
    Home,
    /// Show the Feeds screen.
    Discover,
    /// Another Home tab is open now: show its timeline.
    Selected,
    /// The open tab was clicked again: show what is new.
    ShowNew,
    SignInAgain,
}
