//! A fake Bluesky for the feeds feature's tests.

use crate::api::ApiError;
use crate::api::service::FeedsApi;
use crate::model::{AtUri, FeedGenerator};
use serde_json::json;

/// Knows the feeds and lists it was given; anything else does not exist.
#[derive(Clone, Default)]
pub struct Fake {
    /// (uri, name) of feeds that exist.
    pub feeds: Vec<(&'static str, &'static str)>,
    /// (uri, name) of lists that exist.
    pub lists: Vec<(&'static str, &'static str)>,
}

pub fn generator(uri: &str, name: &str) -> FeedGenerator {
    serde_json::from_value(json!({
        "uri": uri,
        "displayName": name,
        "avatar": format!("https://a/{name}.jpg"),
    }))
    .unwrap()
}

impl FeedsApi for Fake {
    fn feed_generators(
        &self,
        uris: &[AtUri],
    ) -> impl Future<Output = Result<Vec<FeedGenerator>, ApiError>> + Send {
        let found: Vec<FeedGenerator> = self
            .feeds
            .iter()
            .filter(|(u, _)| uris.iter().any(|x| x.as_str() == *u))
            .map(|(u, n)| generator(u, n))
            .collect();
        async move { Ok(found) }
    }
    fn list_name(&self, uri: &str) -> impl Future<Output = Result<String, ApiError>> + Send {
        let name = self
            .lists
            .iter()
            .find(|(u, _)| *u == uri)
            .map(|(_, n)| n.to_string());
        async move { name.ok_or_else(|| ApiError::NotFound("List not found".into())) }
    }
    fn suggested_feeds(&self) -> impl Future<Output = Result<Vec<FeedGenerator>, ApiError>> + Send {
        let all: Vec<FeedGenerator> = self.feeds.iter().map(|(u, n)| generator(u, n)).collect();
        async move { Ok(all) }
    }
}
