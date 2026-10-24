//! Lists of posts that aren't feeds: your bookmarks and the quotes of a post.

use super::{AnyAgent, ApiError, Client, PAGE_SIZE, conv, lenient_items, xrpc_err};
use crate::model::{AtUri, FeedItem, Page};
use serde_json::{Value, json};

/// The order quotes come in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuoteSort {
    /// Most liked first.
    Top,
    /// Newest first.
    Latest,
}

impl QuoteSort {
    fn as_str(self) -> &'static str {
        match self {
            QuoteSort::Top => "top",
            QuoteSort::Latest => "latest",
        }
    }
}

impl Client {
    /// The account's bookmarked posts, newest first. Deleted or blocked ones are skipped.
    pub async fn bookmarks(&self, cursor: Option<String>) -> Result<Page<FeedItem>, ApiError> {
        let out =
            xrpc!(self, |a| a.api.app.bsky.bookmark.get_bookmarks(conv(
                json!({"limit": PAGE_SIZE, "cursor": cursor})
            )?));
        let items: Vec<Value> = conv(&out.bookmarks)?;
        let posts: Vec<Value> = items
            .into_iter()
            .filter_map(|b| b.get("item").cloned())
            .map(|post| json!({ "post": post }))
            .collect();
        Ok(Page {
            items: lenient_items(posts),
            cursor: out.data.cursor.clone(),
        })
    }

    /// A page of the posts quoting `uri`.
    pub async fn quotes(
        &self,
        uri: &AtUri,
        sort: QuoteSort,
        cursor: Option<String>,
    ) -> Result<Page<FeedItem>, ApiError> {
        // atrium-api doesn't know `sort` yet; it is sent all the same.
        let params = json!({
            "uri": uri, "limit": PAGE_SIZE, "cursor": cursor, "sort": sort.as_str()
        });
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .feed
            .get_quotes(conv(params.clone())?));
        Ok(Page {
            items: lenient_items(feed_items(conv(&out.posts)?)),
            cursor: out.data.cursor.clone(),
        })
    }
}

/// Posts as timeline items.
fn feed_items(posts: Vec<Value>) -> Vec<Value> {
    posts
        .into_iter()
        .map(|post| json!({ "post": post }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_become_timeline_items() {
        let items: Vec<Value> = feed_items(vec![json!({"uri": "at://a"})]);
        assert_eq!(items, [json!({"post": {"uri": "at://a"}})]);
        assert_eq!(QuoteSort::Top.as_str(), "top");
    }
}
