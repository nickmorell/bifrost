//! The calls behind reopening a draft and sending a scheduled one.

use crate::api::{self, ApiError, Client, LinkCard, PostDraft};
use crate::model::{AtUri, PostView};

/// What reopening or sending a draft needs.
pub trait DraftsApi: Clone + Send + Sync + 'static {
    /// The posts at `uris` that still exist.
    fn posts(&self, uris: &[AtUri])
    -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send;
    /// The preview card for a link.
    fn link_card(&self, url: &str) -> impl Future<Output = Result<LinkCard, ApiError>> + Send;
    /// Sends the post; gives back its address.
    fn create_post(&self, draft: PostDraft)
    -> impl Future<Output = Result<AtUri, ApiError>> + Send;
}

impl DraftsApi for Client {
    fn posts(
        &self,
        uris: &[AtUri],
    ) -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send {
        Client::posts(self, uris)
    }
    fn link_card(&self, url: &str) -> impl Future<Output = Result<LinkCard, ApiError>> + Send {
        api::link_card(url)
    }
    fn create_post(
        &self,
        draft: PostDraft,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::create_post(self, draft)
    }
}
