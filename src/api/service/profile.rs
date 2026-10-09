//! The calls behind profiles.

use crate::api::{ApiError, Client, ProfileEdit};
use crate::model::{FeedGenerator, ListInfo, Profile};

/// Someone's profile with the feeds and lists they made, and saving your own. Each
/// call returns a future that can move to another thread, so it can run as an iced
/// task.
pub trait ProfileApi: Clone + Send + Sync + 'static {
    /// The profile of `actor`, a DID or handle.
    fn profile(&self, actor: &str) -> impl Future<Output = Result<Profile, ApiError>> + Send;
    /// The custom feeds `actor` made.
    fn actor_feeds(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<FeedGenerator>, ApiError>> + Send;
    /// The lists `actor` made.
    fn actor_lists(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<ListInfo>, ApiError>> + Send;
    /// Saves your display name, bio, profile picture and banner.
    fn save_profile(&self, edit: ProfileEdit) -> impl Future<Output = Result<(), ApiError>> + Send;
}

impl ProfileApi for Client {
    fn profile(&self, actor: &str) -> impl Future<Output = Result<Profile, ApiError>> + Send {
        // The call is large; boxing keeps every task that awaits it small.
        Box::pin(Client::profile(self, actor))
    }
    fn actor_feeds(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<FeedGenerator>, ApiError>> + Send {
        Client::actor_feeds(self, actor)
    }
    fn actor_lists(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<ListInfo>, ApiError>> + Send {
        Client::actor_lists(self, actor)
    }
    fn save_profile(&self, edit: ProfileEdit) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::save_profile(self, edit)
    }
}
