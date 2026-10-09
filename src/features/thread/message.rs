//! What the thread feature is told, and what it tells the app.

use crate::api::ApiError;
use crate::model::{AtUri, ThreadNode};

#[derive(Debug, Clone)]
pub enum Msg {
    /// Fetch the thread around this post.
    Load(AtUri),
    Loaded(AtUri, Result<ThreadNode, ApiError>),
    /// "Continue thread" under a reply nested too deep to show here.
    Open(AtUri),
    /// A row was drawn this tall.
    Measured(u64, f32),
}

/// What the app has to do for the thread feature.
#[derive(Debug)]
pub enum Event {
    /// This thread arrived; its posts' pictures can be loaded.
    Loaded(AtUri),
    /// Loading failed, perhaps because the session ended.
    Failed(ApiError),
    /// Open the thread around this post.
    Open(AtUri),
}
