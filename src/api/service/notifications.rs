//! The calls behind notifications.

use crate::api::{ApiError, Client};
use crate::model::{AtUri, Notification, Page, PostView};

/// Your notifications, the posts they are about, and how many are unseen. Each call
/// returns a future that can move to another thread, so it can run as an iced task.
pub trait NotificationsApi: Clone + Send + Sync + 'static {
    /// A page of notifications, newest first.
    fn notifications(
        &self,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<Notification>, ApiError>> + Send;
    fn posts(&self, uris: &[AtUri])
    -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send;
    /// How many notifications arrived since they were last seen.
    fn unread_count(&self) -> impl Future<Output = Result<u64, ApiError>> + Send;
    /// Marks every notification up to now as seen.
    fn mark_notifications_seen(&self) -> impl Future<Output = Result<(), ApiError>> + Send;
}

impl NotificationsApi for Client {
    fn notifications(
        &self,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<Notification>, ApiError>> + Send {
        Client::notifications(self, cursor)
    }
    fn posts(
        &self,
        uris: &[AtUri],
    ) -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send {
        Client::posts(self, uris)
    }
    fn unread_count(&self) -> impl Future<Output = Result<u64, ApiError>> + Send {
        Client::unread_count(self)
    }
    fn mark_notifications_seen(&self) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::mark_notifications_seen(self)
    }
}
