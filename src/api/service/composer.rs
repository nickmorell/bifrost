//! The calls behind writing a post.

use crate::api::{self, ApiError, Client, LinkCard, PostDraft};
use crate::model::{AtUri, Author};

/// What the composer looks up while you write. Each call returns a future that can
/// move to another thread, so it can run as an iced task.
pub trait ComposerApi: Clone + Send + Sync + 'static {
    /// People whose name or handle starts with `q`, for @mention suggestions.
    fn typeahead(&self, q: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send;
    /// The preview card for a link.
    fn link_card(&self, url: &str) -> impl Future<Output = Result<LinkCard, ApiError>> + Send;
    /// Sends the post; gives back its address.
    fn create_post(&self, draft: PostDraft)
    -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    /// Sends the posts as a thread; gives back the first one's address.
    fn create_thread(
        &self,
        posts: Vec<PostDraft>,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
}

impl ComposerApi for Client {
    fn typeahead(&self, q: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        Client::typeahead(self, q)
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
    fn create_thread(
        &self,
        posts: Vec<PostDraft>,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::create_thread(self, posts)
    }
}
