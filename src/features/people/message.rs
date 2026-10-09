//! What a list of accounts is told, and what it asks of the app.

use crate::api::ApiError;
use crate::model::{Author, Did, Page, PeopleKind};

#[derive(Debug, Clone)]
pub enum Msg {
    /// The list is shown: load it unless it is loaded.
    Open(String, PeopleKind),
    /// Load the list again from the start.
    Refresh(String, PeopleKind),
    /// Load the next page, if there is one.
    More(String, PeopleKind),
    /// Try again after a page failed to load.
    Retry(String, PeopleKind),
    /// A page arrived; `true` when it goes after the ones loaded.
    Loaded(String, PeopleKind, bool, Result<Page<Author>, ApiError>),
    /// An account in the list was clicked.
    Profile(Did),
    Mute(Did, bool),
    Block(Did, bool),
    SignInAgain,
}

/// What the app has to do for the list.
#[derive(Debug)]
pub enum Event {
    /// Load these profile pictures.
    Pictures(Vec<String>),
    /// A page failed to load; the list shows why.
    Failed(ApiError),
    OpenProfile(Did),
    /// Mute (`true`) or unmute the account.
    Mute(Did, bool),
    /// Block (`true`) or unblock the account.
    Block(Did, bool),
    SignInAgain,
}
