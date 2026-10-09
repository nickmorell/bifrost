//! Feeds, threads, feed generators and trending topics.

use super::{AtUri, Author, PostView};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "$type")]
pub enum FeedReason {
    #[serde(rename = "app.bsky.feed.defs#reasonRepost")]
    Repost { by: Box<Author> },
    #[serde(rename = "app.bsky.feed.defs#reasonPin")]
    Pin,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "$type")]
pub enum MaybePost {
    #[serde(rename = "app.bsky.feed.defs#postView")]
    Post(Box<PostView>),
    #[serde(other)]
    Unavailable,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedReplyContext {
    pub parent: MaybePost,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedItem {
    pub post: PostView,
    pub reason: Option<FeedReason>,
    pub reply: Option<FeedReplyContext>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "$type")]
pub enum ThreadNode {
    #[serde(rename = "app.bsky.feed.defs#threadViewPost")]
    Post(Box<ThreadPost>),
    #[serde(rename = "app.bsky.feed.defs#notFoundPost")]
    NotFound,
    #[serde(rename = "app.bsky.feed.defs#blockedPost")]
    Blocked,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThreadPost {
    pub post: PostView,
    pub parent: Option<ThreadNode>,
    #[serde(default)]
    pub replies: Vec<ThreadNode>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedGenerator {
    pub uri: AtUri,
    pub display_name: String,
    pub avatar: Option<String>,
    pub description: Option<String>,
}

/// A trending topic. `link` is a path on bsky.app: a search or a feed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trend {
    pub topic: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub link: String,
}

/// Where tapping a trend goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrendTarget {
    Search(String),
    /// A feed by its owner's handle (or DID) and record key.
    Feed(String, String),
}

impl Trend {
    pub fn name(&self) -> &str {
        self.display_name
            .as_deref()
            .filter(|n| !n.is_empty())
            .unwrap_or(&self.topic)
    }

    pub fn target(&self) -> TrendTarget {
        let path = self.link.split('?').next().unwrap_or("");
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        if let ["profile", owner, "feed", rkey] = parts.as_slice() {
            return TrendTarget::Feed((*owner).into(), (*rkey).into());
        }
        let query = self.link.split_once('?').and_then(|(_, q)| {
            url::form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| k == "q")
                .map(|(_, v)| v.into_owned())
        });
        TrendTarget::Search(
            query
                .filter(|q| !q.is_empty())
                .unwrap_or_else(|| self.topic.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::model::*;

    #[test]
    fn trend_targets() {
        let t = |link: &str| {
            Trend {
                topic: "Topic".into(),
                display_name: None,
                link: link.into(),
            }
            .target()
        };
        assert_eq!(
            t("/profile/trending.bsky.app/feed/abc"),
            TrendTarget::Feed("trending.bsky.app".into(), "abc".into())
        );
        assert_eq!(
            t("/search?q=%23rust+lang"),
            TrendTarget::Search("#rust lang".into())
        );
        assert_eq!(t(""), TrendTarget::Search("Topic".into()));
    }
}
