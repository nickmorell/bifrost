//! What the profile screen and the "Edit profile" dialog send and ask for.

use super::edit;
use super::tab::Tab;
use crate::api::ApiError;
use crate::model::{Did, FeedGenerator, FeedSource, ListInfo, PeopleKind, Profile};

/// A message for profiles.
#[derive(Debug, Clone)]
pub enum Msg {
    /// The profile of `actor` opened on a tab: load what it shows that is missing.
    Open(String, Tab),
    /// Load the lists `actor` made, unless they are loaded already.
    LoadLists(String),
    /// A profile arrived; boxed, as profiles are large.
    Loaded(String, Box<Result<Profile, ApiError>>),
    FeedsLoaded(String, Result<Vec<FeedGenerator>, ApiError>),
    ListsLoaded(String, Result<Vec<ListInfo>, ApiError>),

    /// A tab was picked.
    Tab(String, Tab),
    /// Follow (`true`) or unfollow.
    Follow(Did, bool),
    /// Mute (`true`) or unmute.
    Mute(Did, bool),
    /// Block (`true`) or unblock.
    Block(Did, bool),
    /// "Message" was pressed.
    Message(Did),
    /// "Lists" was pressed: add the account to lists, by its name.
    AddToLists(Did, String),
    Report(Did),
    /// The follower or following count was pressed.
    People(String, PeopleKind),
    /// A custom feed was opened, with its name.
    OpenFeed(FeedSource, String),
    OpenUrl(String),

    /// "Edit profile" was pressed.
    Edit,
    /// A message for the "Edit profile" dialog.
    Editor(edit::Msg),
}

/// What profiles need from the app around them.
pub struct Ctx<'a> {
    /// Your own profile, once loaded; the dialog edits it.
    pub me: Option<&'a Profile>,
}

/// What the shell has to do for profiles.
#[derive(Debug, Clone)]
pub enum Event {
    /// Load these pictures.
    Pictures(Vec<String>),
    /// A call failed; the shell decides whether that means signing in again.
    Failed(ApiError),
    /// Show this tab instead.
    Tab(String, Tab),
    Follow(Did, bool),
    Mute(Did, bool),
    Block(Did, bool),
    Message(Did),
    AddToLists(Did, String),
    Report(Did),
    People(String, PeopleKind),
    OpenFeed(FeedSource, String),
    OpenUrl(String),
    /// Your profile was saved: say so and load it again.
    Saved,
}
