//! What the notifications feature is told, and what it tells the app.

use super::Filter;
use crate::api::ApiError;
use crate::model::{AtUri, Did, Notification, Page, PostView};

#[derive(Debug, Clone)]
pub enum Msg {
    /// Show the list: reload it if it is stale and mark everything seen. `force`
    /// reloads anyway.
    Open {
        force: bool,
    },
    /// Fetch the newest page, or the next older one when `append`.
    Load {
        append: bool,
    },
    Loaded(bool, Result<(Page<Notification>, Vec<PostView>), ApiError>),
    Filter(Filter),
    /// Open or close the list of everyone in a grouped row, by its newest
    /// notification's URI.
    Expand(String),
    /// A row was drawn this tall.
    Measured(u64, f32),
    /// Ask how many notifications are unseen.
    Count,
    Counted(Result<u64, ApiError>),
    Seen(Result<(), ApiError>),
    OpenProfile(Did),
    OpenThread(AtUri),
    FollowBack(Did),
}

/// What the app tells the feature with each message.
pub struct Ctx {
    /// The list is on screen and nothing covers it, so new arrivals can be pulled in
    /// and marked seen straight away.
    pub showing: bool,
}

/// What the app has to do for the notifications feature.
#[derive(Debug)]
pub enum Event {
    /// A page arrived: load these avatars, and the pictures of these posts.
    Loaded {
        avatars: Vec<String>,
        posts: Vec<AtUri>,
    },
    /// Loading failed: say so, and check whether the session ended.
    Failed(ApiError),
    /// A background call failed: only check whether the session ended.
    Quiet(ApiError),
    /// New notifications arrived while the list is not on screen.
    Arrived,
    /// The filter changed: go back to the top of the list.
    ScrollToTop,
    OpenProfile(Did),
    OpenThread(AtUri),
    FollowBack(Did),
}
