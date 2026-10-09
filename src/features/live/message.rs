//! What the Live screens are told, and what they ask of the app.

use crate::api::ApiError;
use crate::api::service::Channel;
use crate::model::{AtUri, Author, Did, LiveEvent, LiveStream, Page, PastStream};

#[derive(Debug, Clone)]
pub enum Msg {
    /// The list of live streams is shown: load it unless it is loaded.
    OpenBoard,
    /// Load the list again.
    RefreshBoard,
    Board(Result<(Vec<LiveStream>, Vec<Author>), ApiError>),
    /// A streamer's page is shown, by handle or DID: load it, or load it again.
    Open(String),
    /// Load a streamer's page again.
    Refresh(String),
    Loaded(String, Result<Box<Channel>, ApiError>),
    /// Load more past streams.
    MorePast(String),
    /// A page of past streams arrived; `true` when it goes after the ones loaded.
    Past(String, bool, Result<Page<PastStream>, ApiError>),
    /// Something happened in this streamer's stream: chat, viewers, a new title.
    Happened(Did, Box<LiveEvent>),
    /// The chat message being written changed.
    Draft(String, String),
    Send(String),
    Sent(String, Result<AtUri, ApiError>),
    /// Play the live stream.
    Watch(String),
    /// Play a past stream.
    WatchPast(String, AtUri),
    /// A stream in the list was clicked.
    OpenStream(String),
    OpenProfile(Did),
    /// Open the streamer's page on stream.place.
    OpenWeb(String),
    SignInAgain,
}

/// What the app has to do for the Live screens.
#[derive(Debug)]
pub enum Event {
    /// Load these pictures.
    Pictures(Vec<String>),
    OpenChannel(String),
    OpenProfile(Did),
    OpenUrl(String),
    /// Play this HLS playlist in the video player under `key`; `web` is where it plays
    /// in a browser, for when it can't play here.
    Play {
        key: String,
        playlist: String,
        web: String,
    },
    /// Something failed; the screen says what.
    Failed(ApiError),
    SignInAgain,
}
