//! What a feed leaves out, set per feed on this computer: BiFrost's own filters, on
//! top of the Following feed preferences kept on Bluesky.

use super::{Embed, EmbeddedRecord, FeedItem, FeedReason, FeedSource};
use serde::{Deserialize, Serialize};

/// The like counts offered as a minimum.
pub const MIN_LIKES_STEPS: [u64; 4] = [0, 10, 50, 250];

/// What one feed leaves out. The default leaves nothing out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeedFilter {
    pub hide_reposts: bool,
    pub hide_replies: bool,
    pub hide_quotes: bool,
    /// Only posts with pictures, a video or a GIF.
    pub media_only: bool,
    /// Leave out posts with fewer likes than this.
    pub min_likes: u64,
}

impl FeedFilter {
    /// Whether anything is left out.
    pub fn is_on(&self) -> bool {
        *self != Self::default()
    }

    /// Whether an item stays in the feed.
    pub fn keep(&self, item: &FeedItem) -> bool {
        let post = &item.post;
        let embed = post.embed.as_ref();
        !(self.hide_reposts && matches!(item.reason, Some(FeedReason::Repost { .. }))
            || self.hide_replies && post.record.reply.is_some()
            || self.hide_quotes && embed.is_some_and(quotes)
            || self.media_only && !embed.is_some_and(has_media)
            || post.like_count < self.min_likes)
    }
}

/// Whether an embed quotes another post.
fn quotes(e: &Embed) -> bool {
    match e {
        Embed::Record { record } => matches!(record, EmbeddedRecord::Post(_)),
        Embed::RecordWithMedia { record, .. } => matches!(record.record, EmbeddedRecord::Post(_)),
        _ => false,
    }
}

/// Whether an embed carries pictures, a video or a GIF.
fn has_media(e: &Embed) -> bool {
    match e {
        Embed::Images { .. } | Embed::Video { .. } => true,
        Embed::External { external } => external
            .uri
            .split(['?', '#'])
            .next()
            .is_some_and(|u| u.to_ascii_lowercase().ends_with(".gif")),
        Embed::RecordWithMedia { media, .. } => has_media(media),
        _ => false,
    }
}

impl FeedSource {
    /// A name for the feed that stays the same between runs, to keep things by.
    pub fn id(&self) -> &str {
        match self {
            FeedSource::Following => "following",
            FeedSource::Generator(u) | FeedSource::List(u) => u,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(post: serde_json::Value, reason: Option<serde_json::Value>) -> FeedItem {
        let mut v = json!({ "post": post });
        if let Some(r) = reason {
            v["reason"] = r;
        }
        serde_json::from_value(v).unwrap()
    }

    fn post(extra: serde_json::Value) -> serde_json::Value {
        let mut p = json!({
            "uri": "at://did:plc:a/app.bsky.feed.post/1",
            "cid": "c",
            "author": { "did": "did:plc:a", "handle": "a.test" },
            "record": { "text": "hi", "createdAt": "2026-01-01T00:00:00Z" },
            "likeCount": 10,
            "indexedAt": "2026-01-01T00:00:00Z"
        });
        for (k, v) in extra.as_object().unwrap() {
            p[k] = v.clone();
        }
        p
    }

    #[test]
    fn each_filter_leaves_out_its_posts() {
        let plain = item(post(json!({})), None);
        let repost = item(
            post(json!({})),
            Some(json!({
                "$type": "app.bsky.feed.defs#reasonRepost",
                "by": { "did": "did:plc:b", "handle": "b.test" }
            })),
        );
        let reply_ref = json!({ "uri": "at://did:plc:b/app.bsky.feed.post/0", "cid": "c" });
        let reply = item(
            post(json!({ "record": {
                "text": "re", "createdAt": "2026-01-01T00:00:00Z",
                "reply": { "root": reply_ref, "parent": reply_ref }
            }})),
            None,
        );
        let photo = item(
            post(json!({ "embed": {
                "$type": "app.bsky.embed.images#view",
                "images": [{ "thumb": "t", "fullsize": "f", "alt": "" }]
            }})),
            None,
        );

        let none = FeedFilter::default();
        assert!(!none.is_on());
        for i in [&plain, &repost, &reply, &photo] {
            assert!(none.keep(i));
        }
        let f = FeedFilter {
            hide_reposts: true,
            ..none
        };
        assert!(f.is_on() && f.keep(&plain) && !f.keep(&repost));
        let f = FeedFilter {
            hide_replies: true,
            ..none
        };
        assert!(f.keep(&plain) && !f.keep(&reply));
        let f = FeedFilter {
            media_only: true,
            ..none
        };
        assert!(!f.keep(&plain) && f.keep(&photo));
        let f = FeedFilter {
            min_likes: 20,
            ..none
        };
        assert!(!f.keep(&plain));
        assert_eq!(FeedSource::Following.id(), "following");
    }
}
