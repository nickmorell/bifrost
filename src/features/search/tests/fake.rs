//! A fake for the search tests. The tests check what `update` does with each message
//! and how many loads it starts, so the calls are never run and answer nothing.

use crate::api::ApiError;
use crate::api::service::SearchApi;
use crate::model::{Author, Did, FeedGenerator, Page, PostView, Trend};
use std::future::{Ready, ready};

#[derive(Clone, Default)]
pub struct Fake;

fn nothing<T>() -> Ready<Result<T, ApiError>> {
    ready(Err(ApiError::Other("not run in tests".into())))
}

impl SearchApi for Fake {
    fn search_posts(
        &self,
        _: &str,
        _: &str,
        _: Option<String>,
    ) -> impl Future<Output = Result<Page<PostView>, ApiError>> + Send {
        nothing()
    }
    fn search_actors(
        &self,
        _: &str,
        _: Option<String>,
    ) -> impl Future<Output = Result<Page<Author>, ApiError>> + Send {
        nothing()
    }
    fn search_feeds(
        &self,
        _: &str,
        _: Option<String>,
    ) -> impl Future<Output = Result<Page<FeedGenerator>, ApiError>> + Send {
        nothing()
    }
    fn typeahead(&self, _: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        nothing()
    }
    fn trending(&self) -> impl Future<Output = Result<Vec<Trend>, ApiError>> + Send {
        nothing()
    }
    fn suggested_people(&self) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        nothing()
    }
    fn resolve_handle(&self, _: &str) -> impl Future<Output = Result<Did, ApiError>> + Send {
        nothing()
    }
}
