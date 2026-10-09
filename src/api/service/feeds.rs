//! The calls behind your feeds: the Home tabs and the Feeds screen.

use crate::api::{ApiError, Client};
use crate::model::{AtUri, FeedGenerator};

/// Feed and list names, and the feeds Bluesky suggests. Each call returns a future
/// that can move to another thread, so it can run as an iced task.
pub trait FeedsApi: Clone + Send + Sync + 'static {
    /// The feeds at `uris` that still exist.
    fn feed_generators(
        &self,
        uris: &[AtUri],
    ) -> impl Future<Output = Result<Vec<FeedGenerator>, ApiError>> + Send;
    /// The name of the list at `uri`.
    fn list_name(&self, uri: &str) -> impl Future<Output = Result<String, ApiError>> + Send;
    fn suggested_feeds(&self) -> impl Future<Output = Result<Vec<FeedGenerator>, ApiError>> + Send;
}

impl FeedsApi for Client {
    fn feed_generators(
        &self,
        uris: &[AtUri],
    ) -> impl Future<Output = Result<Vec<FeedGenerator>, ApiError>> + Send {
        Client::feed_generators(self, uris)
    }
    fn list_name(&self, uri: &str) -> impl Future<Output = Result<String, ApiError>> + Send {
        Client::list_name(self, uri)
    }
    fn suggested_feeds(&self) -> impl Future<Output = Result<Vec<FeedGenerator>, ApiError>> + Send {
        Client::suggested_feeds(self)
    }
}
