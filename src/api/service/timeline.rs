//! The calls behind timelines.

use crate::api::{ApiError, AuthorFilter, Client, QuoteSort};
use crate::model::{AtUri, FeedItem, FeedSource, Page};

/// Pages of posts: a feed or list, someone's posts, and your bookmarks. Each call
/// returns a future that can move to another thread, so it can run as an iced task.
pub trait TimelineApi: Clone + Send + Sync + 'static {
    /// A page of a feed or list, continuing from `cursor`.
    fn feed(
        &self,
        source: &FeedSource,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send;
    /// The newest few posts of a feed or list, to check for new ones.
    fn feed_peek(
        &self,
        source: &FeedSource,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send;
    /// A page of what `actor` posted, replied or liked, by `filter`.
    fn author_feed(
        &self,
        actor: &str,
        filter: AuthorFilter,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send;
    /// A page of your bookmarks.
    fn bookmarks(
        &self,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send;
    /// A page of the posts quoting `uri`.
    fn quotes(
        &self,
        uri: &AtUri,
        sort: QuoteSort,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send;
}

impl TimelineApi for Client {
    fn feed(
        &self,
        source: &FeedSource,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        Client::feed(self, source, cursor)
    }
    fn feed_peek(
        &self,
        source: &FeedSource,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        Client::feed_peek(self, source)
    }
    fn author_feed(
        &self,
        actor: &str,
        filter: AuthorFilter,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        Client::author_feed(self, actor, filter, cursor)
    }
    fn bookmarks(
        &self,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        Client::bookmarks(self, cursor)
    }
    fn quotes(
        &self,
        uri: &AtUri,
        sort: QuoteSort,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        Client::quotes(self, uri, sort, cursor)
    }
}
