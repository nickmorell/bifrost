//! The Following feed's filters.

use crate::model::{Embed, FeedItem, FeedReason, FeedReplyContext, MaybePost};

/// The "Following feed preferences" from the official app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowingPrefs {
    pub hide_replies: bool,
    /// Only show replies to people you follow.
    pub hide_replies_by_unfollowed: bool,
    /// Hide replies with fewer likes than this.
    pub min_reply_likes: u64,
    pub hide_reposts: bool,
    pub hide_quote_posts: bool,
}

impl Default for FollowingPrefs {
    fn default() -> Self {
        // The official app's defaults.
        Self {
            hide_replies: false,
            hide_replies_by_unfollowed: true,
            min_reply_likes: 0,
            hide_reposts: false,
            hide_quote_posts: false,
        }
    }
}

impl FollowingPrefs {
    /// Whether an item belongs in your Following feed, `me` being your DID.
    pub fn keep(&self, me: &str, item: &FeedItem) -> bool {
        let post = &item.post;
        if self.hide_reposts && matches!(item.reason, Some(FeedReason::Repost { .. })) {
            return false;
        }
        if self.hide_quote_posts
            && matches!(
                post.embed,
                Some(Embed::Record { .. } | Embed::RecordWithMedia { .. })
            )
        {
            return false;
        }
        let Some(reply) = &post.record.reply else {
            return true;
        };
        // A repost of a reply was chosen by someone you follow; and your own replies
        // always show.
        if item.reason.is_some() || post.author.did == me {
            return true;
        }
        let parent_did = parse_did(&reply.parent.uri);
        // Someone continuing their own thread is not really a reply.
        let self_thread = parent_did == Some(post.author.did.as_str());
        if self.hide_replies && !self_thread {
            return false;
        }
        if self.min_reply_likes > 0 && !self_thread && post.like_count < self.min_reply_likes {
            return false;
        }
        if self.hide_replies_by_unfollowed && !self_thread {
            let parent = match &item.reply {
                Some(FeedReplyContext {
                    parent: MaybePost::Post(p),
                }) => Some(&p.author),
                _ => None,
            };
            // Unknown parents are kept rather than guessed at.
            if let Some(a) = parent
                && a.did != me
                && a.viewer.following.is_none()
            {
                return false;
            }
        }
        true
    }
}

/// The DID in an `at://did/collection/rkey` URI.
fn parse_did(uri: &str) -> Option<&str> {
    uri.strip_prefix("at://")?.split('/').next()
}

#[cfg(test)]
mod tests {
    use crate::model::*;
    use serde_json::json;

    #[test]
    fn following_feed_prefs() {
        let prefs = Preferences::from_json(&[json!({
            "$type": "app.bsky.actor.defs#feedViewPref", "feed": "home",
            "hideReposts": true, "hideRepliesByLikeCount": 2
        })]);
        let f = prefs.following;
        assert!(f.hide_reposts && f.hide_replies_by_unfollowed && f.min_reply_likes == 2);

        let item = |author: &str, parent: Option<(&str, bool)>, likes: u64, repost: bool| {
            let mut v = json!({"post": {
                "uri": format!("at://{author}/app.bsky.feed.post/1"), "cid": "c",
                "author": {"did": author, "handle": "x.test"},
                "record": {"text": "hi"}, "likeCount": likes,
            }});
            if let Some((p, followed)) = parent {
                v["post"]["record"]["reply"] = json!({
                    "root": {"uri": format!("at://{p}/app.bsky.feed.post/0"), "cid": "c"},
                    "parent": {"uri": format!("at://{p}/app.bsky.feed.post/0"), "cid": "c"},
                });
                let viewer = if followed {
                    json!({"following": "at://f"})
                } else {
                    json!({})
                };
                v["reply"] = json!({"parent": {
                    "$type": "app.bsky.feed.defs#postView",
                    "uri": format!("at://{p}/app.bsky.feed.post/0"), "cid": "c",
                    "author": {"did": p, "handle": "p.test", "viewer": viewer},
                    "record": {"text": "parent"},
                }});
            }
            if repost {
                v["reason"] = json!({"$type": "app.bsky.feed.defs#reasonRepost",
                    "by": {"did": "did:r", "handle": "r.test"}});
            }
            serde_json::from_value::<FeedItem>(v).unwrap()
        };
        let me = "did:me";
        assert!(f.keep(me, &item("did:a", None, 0, false)));
        assert!(!f.keep(me, &item("did:a", None, 0, true)), "reposts hidden");
        assert!(f.keep(me, &item("did:a", Some(("did:b", true)), 5, false)));
        assert!(
            !f.keep(me, &item("did:a", Some(("did:b", false)), 5, false)),
            "unfollowed"
        );
        assert!(
            !f.keep(me, &item("did:a", Some(("did:b", true)), 1, false)),
            "few likes"
        );
        assert!(
            f.keep(me, &item("did:a", Some(("did:a", false)), 0, false)),
            "self thread"
        );
        assert!(
            f.keep(me, &item("did:a", Some((me, false)), 5, false)),
            "reply to me"
        );
    }
}
