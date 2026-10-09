//! The call behind the thread screen.

use crate::api::{ApiError, Client};
use crate::model::{AtUri, ThreadNode};

/// One post with the posts above it and its replies. The call returns a future that
/// can move to another thread, so it can run as an iced task.
pub trait ThreadApi: Clone + Send + Sync + 'static {
    /// The thread around the post at `uri`: its parents up to the first post, and its
    /// replies.
    fn thread(&self, uri: &AtUri) -> impl Future<Output = Result<ThreadNode, ApiError>> + Send;
}

impl ThreadApi for Client {
    fn thread(&self, uri: &AtUri) -> impl Future<Output = Result<ThreadNode, ApiError>> + Send {
        Client::thread(self, uri)
    }
}
