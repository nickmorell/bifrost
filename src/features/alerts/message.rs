//! What the alerts feature reacts to and hands back.

use crate::api::ApiError;
use crate::model::{Alerts as Kinds, Convo, Did, Notification, Page, PostView, Preferences};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum Msg {
    /// You saw everything up to `0` (the window got focus, or a session started).
    Seen(DateTime<Utc>),
    /// Look for anything new to tell you about.
    Check,
    Notifications(Result<Page<Notification>, ApiError>),
    /// New notifications that need their post's text, and those posts.
    Posts(Vec<Notification>, Result<Vec<PostView>, ApiError>),
    Convos(Result<Page<Convo>, ApiError>),
}

/// What the feature needs to know from the app.
pub struct Ctx<'a> {
    /// Which kinds to tell you about.
    pub kinds: &'a Kinds,
    pub me: &'a Did,
    /// Your muted words, which keep a notice from showing.
    pub prefs: &'a Preferences,
}

/// One notification for the app to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub body: String,
}
