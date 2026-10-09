//! The calls behind search and the Explore page.

use crate::api::{ApiError, Client};
use crate::model::{Author, Did, FeedGenerator, Page, PostView, Trend};

/// Searching posts, people and feeds, and what the Explore page shows before anything
/// is searched. Each call returns a future that can move to another thread, so it can
/// run as an iced task.
pub trait SearchApi: Clone + Send + Sync + 'static {
    /// Posts matching `q`, ordered by `sort`: "top" or "latest".
    fn search_posts(
        &self,
        q: &str,
        sort: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<PostView>, ApiError>> + Send;
    /// Accounts matching `q`.
    fn search_actors(
        &self,
        q: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<Author>, ApiError>> + Send;
    /// Feeds whose name or description match `q`.
    fn search_feeds(
        &self,
        q: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedGenerator>, ApiError>> + Send;
    /// A few accounts whose name or handle starts with `q`, while typing.
    fn typeahead(&self, q: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send;
    /// What people are talking about right now.
    fn trending(&self) -> impl Future<Output = Result<Vec<Trend>, ApiError>> + Send;
    /// Accounts Bluesky suggests following.
    fn suggested_people(&self) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send;
    /// The DID behind a handle.
    fn resolve_handle(&self, handle: &str) -> impl Future<Output = Result<Did, ApiError>> + Send;
}

impl SearchApi for Client {
    fn search_posts(
        &self,
        q: &str,
        sort: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<PostView>, ApiError>> + Send {
        Client::search_posts(self, q, sort, cursor)
    }
    fn search_actors(
        &self,
        q: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<Author>, ApiError>> + Send {
        Client::search_actors(self, q, cursor)
    }
    fn search_feeds(
        &self,
        q: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedGenerator>, ApiError>> + Send {
        Client::search_feeds(self, q, cursor)
    }
    fn typeahead(&self, q: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        Client::typeahead(self, q)
    }
    fn trending(&self) -> impl Future<Output = Result<Vec<Trend>, ApiError>> + Send {
        Client::trending(self)
    }
    fn suggested_people(&self) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        Client::suggested_people(self)
    }
    fn resolve_handle(&self, handle: &str) -> impl Future<Output = Result<Did, ApiError>> + Send {
        Client::resolve_handle(self, handle)
    }
}
