//! What the lists feature takes in ([`Msg`]) and what it asks of the app ([`Event`]).

use super::state::ListTab;
use crate::api::ApiError;
use crate::model::{
    AtUri, Author, Did, FeedSource, ListItem, ListPage, Membership, StarterPack, StarterPackBasic,
};

#[derive(Debug, Clone)]
pub enum Msg {
    OpenList(AtUri),
    OpenPack(AtUri),
    Loaded(AtUri, Result<Box<ListPage>, ApiError>, bool),
    More(AtUri),
    Tab(AtUri, ListTab),
    Query(AtUri, String),
    Suggest(AtUri, u64),
    Results(AtUri, u64, Result<Vec<Author>, ApiError>),
    Add(AtUri, Author),
    Added(AtUri, Author, Result<AtUri, ApiError>),
    Remove(AtUri, ListItem),
    Removed(AtUri, ListItem, Result<(), ApiError>),
    Mute(AtUri, bool),
    Muted(AtUri, bool, Result<(), ApiError>),
    Block(AtUri, Option<AtUri>),
    Blocked(AtUri, Result<Option<AtUri>, ApiError>),
    PackLoaded(AtUri, Result<Box<StarterPack>, ApiError>),
    ActorPacks(String, Result<Vec<StarterPackBasic>, ApiError>),
    FollowAll(AtUri),
    FollowedAll(AtUri, Result<Vec<(Did, AtUri)>, ApiError>),
    PacksTab(bool),

    NewList(bool),
    NewPack,
    Edit(AtUri),
    EditPack(AtUri),
    Name(String),
    Description(String),
    ToggleFeed(AtUri),
    Save,
    Saved(Result<AtUri, ApiError>),
    CloseEditor,

    Delete(AtUri),
    ConfirmDelete,
    CancelDelete,
    Deleted(AtUri, Result<(), ApiError>),

    AddTo(Did, String),
    Memberships(Did, Result<Vec<Membership>, ApiError>),
    Toggle(AtUri),
    Toggled(AtUri, Result<Option<AtUri>, ApiError>),
    CloseAddTo,

    /// Pressed in a view; passed on to the app as the [`Event`] of the same name.
    OpenProfile(Did),
    OpenUrl(String),
    OpenFeed(FeedSource, String),
    Retry,
    SignInAgain,
}

/// Something the app has to do for the lists feature, in the order given.
#[derive(Debug)]
pub enum Event {
    OpenList(AtUri),
    OpenPack(AtUri),
    OpenProfile(Did),
    OpenUrl(String),
    OpenFeed(FeedSource, String),
    /// Try loading what failed again.
    Retry,
    /// The session ended; go to sign-in.
    SignInAgain,
    Toast(String),
    /// Something failed: tell the user.
    Error(ApiError),
    /// Something failed to load and the screen already says so; only a lost session
    /// needs the app.
    ErrorShown(ApiError),
    /// Fetch these pictures.
    Images(Vec<String>),
    /// A list screen switched tabs, so its timeline may need loading.
    TabChanged,
    /// Your own lists changed; forget the app's copy.
    MyListsChanged,
    /// The Lists screen is showing and needs your lists.
    NeedMyLists,
    /// A list or pack was deleted: leave its page if it is open, else refresh.
    Deleted(AtUri),
}

/// What the feature needs to know about the signed-in account.
#[derive(Debug, Clone, Copy)]
pub struct Ctx<'a> {
    pub me: &'a Did,
    /// Your display name, for naming a new starter pack.
    pub my_name: Option<&'a str>,
}
