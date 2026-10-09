//! The writes behind what you do to posts and accounts.

use crate::api::{ApiError, Client};
use crate::model::{AtUri, Interaction, StrongRef};

/// Likes, reposts, bookmarks, deleting, pinning, muting threads, following, muting and
/// blocking. Each call returns a future that can move to another thread, so it can run
/// as an iced task.
pub trait ActionsApi: Clone + Send + Sync + 'static {
    fn like(&self, post: &StrongRef) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    fn repost(&self, post: &StrongRef) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    /// Deletes one of your records: a post, like, repost, follow or block.
    fn delete(&self, uri: &AtUri) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn bookmark(&self, post: &StrongRef) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn unbookmark(&self, uri: &AtUri) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// Mutes (`true`) or unmutes the thread whose first post is `root`.
    fn mute_thread(
        &self,
        root: &AtUri,
        mute: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// Pins a post to your profile, or unpins it with `None`.
    fn pin(&self, post: Option<StrongRef>) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn follow(&self, did: &str) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    fn block(&self, did: &str) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    /// Mutes (`true`) or unmutes an account.
    fn mute(&self, did: &str, mute: bool) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// Who can reply to and quote one of your posts.
    fn post_interaction(
        &self,
        uri: &str,
    ) -> impl Future<Output = Result<Interaction, ApiError>> + Send;
    /// Changes who can reply to and quote one of your posts; replies only apply to
    /// the first post of a thread.
    fn set_post_interaction(
        &self,
        uri: &str,
        value: Interaction,
        is_root: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
}

impl ActionsApi for Client {
    fn like(&self, post: &StrongRef) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::like(self, post)
    }
    fn repost(&self, post: &StrongRef) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::repost(self, post)
    }
    fn delete(&self, uri: &AtUri) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::delete(self, uri)
    }
    fn bookmark(&self, post: &StrongRef) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::bookmark(self, post)
    }
    fn unbookmark(&self, uri: &AtUri) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::unbookmark(self, uri)
    }
    async fn mute_thread(&self, root: &AtUri, mute: bool) -> Result<(), ApiError> {
        if mute {
            Client::mute_thread(self, root).await
        } else {
            Client::unmute_thread(self, root).await
        }
    }
    fn pin(&self, post: Option<StrongRef>) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::pin_post(self, post)
    }
    fn follow(&self, did: &str) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::follow(self, did)
    }
    fn block(&self, did: &str) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::block(self, did)
    }
    async fn mute(&self, did: &str, mute: bool) -> Result<(), ApiError> {
        if mute {
            Client::mute(self, did).await
        } else {
            Client::unmute(self, did).await
        }
    }
    fn post_interaction(
        &self,
        uri: &str,
    ) -> impl Future<Output = Result<Interaction, ApiError>> + Send {
        Client::post_interaction(self, uri)
    }
    fn set_post_interaction(
        &self,
        uri: &str,
        value: Interaction,
        is_root: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::set_post_interaction(self, uri, value, is_root)
    }
}
