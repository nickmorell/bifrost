//! Who can reply to and quote a post (threadgates and postgates).

use super::{AtUri, ListInfo};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct ThreadgateView {
    #[serde(default)]
    pub record: Value,
    /// Lists named in the rules, with their names.
    #[serde(default)]
    pub lists: Vec<ListInfo>,
}

/// Who can reply to a thread you start, and whether your post can be quoted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interaction {
    /// `None`: everyone can reply. `Some` with nothing allowed: nobody can.
    pub replies: Option<ReplyRules>,
    pub quotes: bool,
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            replies: None,
            quotes: true,
        }
    }
}

/// Who besides you may reply when replies are limited.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReplyRules {
    pub mentioned: bool,
    pub followers: bool,
    pub following: bool,
    /// Members of these lists (by URI).
    pub lists: Vec<AtUri>,
}

const MENTION_RULE: &str = "app.bsky.feed.threadgate#mentionRule";
const FOLLOWER_RULE: &str = "app.bsky.feed.threadgate#followerRule";
const FOLLOWING_RULE: &str = "app.bsky.feed.threadgate#followingRule";
const LIST_RULE: &str = "app.bsky.feed.threadgate#listRule";
const DISABLE_RULE: &str = "app.bsky.feed.postgate#disableRule";

impl ReplyRules {
    pub fn is_nobody(&self) -> bool {
        !self.mentioned && !self.followers && !self.following && self.lists.is_empty()
    }

    /// The threadgate `allow` array.
    pub fn to_json(&self) -> Vec<Value> {
        let mut out = Vec::new();
        for (on, ty) in [
            (self.mentioned, MENTION_RULE),
            (self.followers, FOLLOWER_RULE),
            (self.following, FOLLOWING_RULE),
        ] {
            if on {
                out.push(serde_json::json!({ "$type": ty }));
            }
        }
        for l in &self.lists {
            out.push(serde_json::json!({"$type": LIST_RULE, "list": l}));
        }
        out
    }

    pub fn from_json(allow: &[Value]) -> Self {
        let mut out = Self::default();
        for rule in allow {
            match rule
                .get("$type")
                .and_then(Value::as_str)
                .unwrap_or_default()
            {
                MENTION_RULE => out.mentioned = true,
                FOLLOWER_RULE => out.followers = true,
                FOLLOWING_RULE => out.following = true,
                LIST_RULE => out
                    .lists
                    .extend(rule.get("list").and_then(Value::as_str).map(AtUri::from)),
                _ => {}
            }
        }
        out
    }
}

impl Interaction {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Reads a threadgate's and a postgate's records; either may be missing.
    pub fn from_records(threadgate: Option<&Value>, postgate: Option<&Value>) -> Self {
        let replies = threadgate.map(|t| {
            ReplyRules::from_json(
                t.get("allow")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            )
        });
        // A threadgate with no `allow` at all lets everyone reply.
        let replies = match (replies, threadgate.and_then(|t| t.get("allow"))) {
            (Some(_), None) => None,
            (r, _) => r,
        };
        Self {
            replies,
            quotes: !postgate.is_some_and(|p| {
                p.get("embeddingRules")
                    .and_then(Value::as_array)
                    .is_some_and(|r| {
                        r.iter()
                            .any(|r| r.get("$type").and_then(Value::as_str) == Some(DISABLE_RULE))
                    })
            }),
        }
    }

    /// The postgate `embeddingRules` array.
    pub fn embedding_rules(&self) -> Vec<Value> {
        if self.quotes {
            Vec::new()
        } else {
            vec![serde_json::json!({ "$type": DISABLE_RULE })]
        }
    }

    /// A short sentence saying who can interact.
    pub fn summary(&self) -> &'static str {
        match (&self.replies, self.quotes) {
            (None, true) => "Everybody can interact",
            (None, false) => "Quotes off",
            (Some(r), true) if r.is_nobody() => "Replies off",
            (Some(r), false) if r.is_nobody() => "Replies and quotes off",
            (Some(_), true) => "Some people can reply",
            (Some(_), false) => "Some people can reply, quotes off",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::*;
    use serde_json::json;

    #[test]
    fn interaction_settings_round_trip() {
        let limited = Interaction {
            replies: Some(ReplyRules {
                mentioned: true,
                following: true,
                lists: vec!["at://did:plc:a/app.bsky.graph.list/1".into()],
                ..Default::default()
            }),
            quotes: false,
        };
        let gate = json!({ "allow": limited.replies.as_ref().unwrap().to_json() });
        let post = json!({ "embeddingRules": limited.embedding_rules() });
        assert_eq!(Interaction::from_records(Some(&gate), Some(&post)), limited);
        // No records at all, or a gate without rules: everyone.
        assert!(Interaction::from_records(None, None).is_default());
        assert!(
            Interaction::from_records(Some(&json!({})), None)
                .replies
                .is_none()
        );
        // An empty allow list: nobody.
        let nobody = Interaction::from_records(Some(&json!({"allow": []})), None);
        assert!(nobody.replies.unwrap().is_nobody());

        let mut prefs = Vec::new();
        PrefEdit::PostInteraction(limited.clone()).apply(&mut prefs);
        assert_eq!(Preferences::from_json(&prefs).interaction, limited);
        PrefEdit::PostInteraction(Interaction::default()).apply(&mut prefs);
        assert!(Preferences::from_json(&prefs).interaction.is_default());
    }
}
