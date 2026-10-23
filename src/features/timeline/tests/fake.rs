//! A fake for the timeline tests. The tests check what `update` does with each message
//! and how many loads it starts, so the calls are never run and answer nothing.

use crate::api::service::TimelineApi;
use crate::api::{ApiError, AuthorFilter, QuoteSort};
use crate::model::{AtUri, FeedItem, FeedSource, Page};
use std::future::{Ready, ready};

#[derive(Clone, Default)]
pub struct Fake;

fn nothing<T>() -> Ready<Result<T, ApiError>> {
    ready(Err(ApiError::Other("not run in tests".into())))
}

impl TimelineApi for Fake {
    fn feed(
        &self,
        _: &FeedSource,
        _: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        nothing()
    }
    fn feed_peek(
        &self,
        _: &FeedSource,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        nothing()
    }
    fn author_feed(
        &self,
        _: &str,
        _: AuthorFilter,
        _: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        nothing()
    }
    fn bookmarks(
        &self,
        _: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        nothing()
    }
    fn quotes(
        &self,
        _: &AtUri,
        _: QuoteSort,
        _: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedItem>, ApiError>> + Send {
        nothing()
    }
}
