//! The calls behind your collections.

use crate::api::{ApiError, Client};
use crate::model::{AtUri, PostView};

/// What showing a collection looks up.
pub trait CollectionsApi: Clone + Send + Sync + 'static {
    /// The posts at `uris` that still exist.
    fn posts(&self, uris: &[AtUri])
    -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send;
}

impl CollectionsApi for Client {
    fn posts(
        &self,
        uris: &[AtUri],
    ) -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send {
        Client::posts(self, uris)
    }
}
