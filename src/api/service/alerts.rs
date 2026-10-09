//! The calls behind OS notifications.

use crate::api::{ApiError, Client};
use crate::model::{AtUri, Convo, Notification, Page, PostView};

/// What is new, for telling you about it while BiFrost is in the background. Each call
/// returns a future that can move to another thread, so it can run as an iced task.
pub trait AlertsApi: Clone + Send + Sync + 'static {
    /// The newest notifications.
    fn notifications(&self) -> impl Future<Output = Result<Page<Notification>, ApiError>> + Send;
    /// The posts notifications are about, for their text.
    fn posts(&self, uris: &[AtUri])
    -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send;
    /// Your chats, the most recently active first.
    fn convos(&self) -> impl Future<Output = Result<Page<Convo>, ApiError>> + Send;
}

impl AlertsApi for Client {
    fn notifications(&self) -> impl Future<Output = Result<Page<Notification>, ApiError>> + Send {
        Client::notifications(self, None)
    }
    fn posts(
        &self,
        uris: &[AtUri],
    ) -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send {
        Client::posts(self, uris)
    }
    fn convos(&self) -> impl Future<Output = Result<Page<Convo>, ApiError>> + Send {
        Client::convos(self, false, None)
    }
}
