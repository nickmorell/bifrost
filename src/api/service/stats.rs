//! The calls behind your stats.

use crate::api::{ApiError, AuthorFilter, Client};
use crate::model::{FeedReason, PostView};

/// Pages of your own posts read to bring the stats up to date.
const PAGES: usize = 3;

/// What the stats screen loads.
pub trait StatsApi: Clone + Send + Sync + 'static {
    /// Your latest posts and replies (not reposts), with their counts as they are now.
    fn my_posts(&self, did: &str) -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send;
}

impl StatsApi for Client {
    async fn my_posts(&self, did: &str) -> Result<Vec<PostView>, ApiError> {
        let mut out = Vec::new();
        let mut cursor = None;
        for _ in 0..PAGES {
            let page = self
                .author_feed(did, AuthorFilter::Replies, cursor.take())
                .await?;
            out.extend(
                page.items
                    .into_iter()
                    .filter(|i| !matches!(i.reason, Some(FeedReason::Repost { .. })))
                    .filter(|i| i.post.author.did.as_str() == did)
                    .map(|i| i.post),
            );
            match page.cursor {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        Ok(out)
    }
}
