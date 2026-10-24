//! The account's preferences (`app.bsky.actor.getPreferences`).

mod edit;
mod following;

pub use edit::PrefEdit;
pub use following::FollowingPrefs;

use super::{AtUri, Interaction, ReplyRules};
use serde_json::Value;

/// One entry of the user's saved feeds, in their chosen order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FeedSource {
    Following,
    Generator(AtUri),
    List(AtUri),
}

#[derive(Debug, Clone)]
pub struct SavedFeed {
    pub source: FeedSource,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelVisibility {
    Ignore,
    Warn,
    Hide,
}

#[derive(Debug, Clone)]
pub struct MutedWord {
    pub value: String,
    /// "content" and/or "tag".
    pub targets: Vec<String>,
}

/// The parts of `app.bsky.actor.getPreferences` BiFrost acts on.
#[derive(Debug, Clone, Default)]
pub struct Preferences {
    pub saved_feeds: Vec<SavedFeed>,
    pub adult_content_enabled: bool,
    /// (label value, labeler DID if scoped to one, visibility)
    pub content_labels: Vec<(String, Option<String>, LabelVisibility)>,
    pub muted_words: Vec<MutedWord>,
    /// What the Following feed leaves out.
    pub following: FollowingPrefs,
    /// Who can reply to and quote new posts, unless changed in the composer.
    pub interaction: Interaction,
    /// Leave out the blue verification checks beside names.
    pub hide_verification_badges: bool,
    /// The preferences exactly as the server sent them, so edits keep everything
    /// BiFrost does not understand.
    pub raw: Vec<Value>,
}

const SAVED_FEEDS: &str = "app.bsky.actor.defs#savedFeedsPrefV2";
const ADULT: &str = "app.bsky.actor.defs#adultContentPref";
const LABEL: &str = "app.bsky.actor.defs#contentLabelPref";
const MUTED_WORDS: &str = "app.bsky.actor.defs#mutedWordsPref";
const POST_INTERACTION: &str = "app.bsky.actor.defs#postInteractionSettingsPref";
const VERIFICATION: &str = "app.bsky.actor.defs#verificationPrefs";

impl LabelVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            LabelVisibility::Ignore => "ignore",
            LabelVisibility::Warn => "warn",
            LabelVisibility::Hide => "hide",
        }
    }
}

impl FeedSource {
    fn pref_item(&self) -> (&'static str, &str) {
        match self {
            FeedSource::Following => ("timeline", "following"),
            FeedSource::Generator(u) => ("feed", u),
            FeedSource::List(u) => ("list", u),
        }
    }

    fn matches(&self, item: &Value) -> bool {
        let (ty, value) = self.pref_item();
        item.get("type").and_then(Value::as_str) == Some(ty)
            && (ty == "timeline" || item.get("value").and_then(Value::as_str) == Some(value))
    }
}

impl Preferences {
    pub fn from_json(prefs: &[Value]) -> Self {
        let mut out = Preferences {
            raw: prefs.to_vec(),
            ..Preferences::default()
        };
        for p in prefs {
            let s = |k: &str| p.get(k).and_then(Value::as_str).map(str::to_string);
            match p.get("$type").and_then(Value::as_str).unwrap_or_default() {
                "app.bsky.actor.defs#savedFeedsPrefV2" => {
                    for item in p
                        .get("items")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        let ty = item.get("type").and_then(Value::as_str).unwrap_or_default();
                        let value = item
                            .get("value")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        let source = match ty {
                            "timeline" => FeedSource::Following,
                            "feed" => FeedSource::Generator(value.into()),
                            "list" => FeedSource::List(value.into()),
                            _ => continue,
                        };
                        let pinned = item.get("pinned").and_then(Value::as_bool).unwrap_or(false);
                        out.saved_feeds.push(SavedFeed { source, pinned });
                    }
                }
                "app.bsky.actor.defs#feedViewPref" if s("feed").as_deref() == Some("home") => {
                    let b = |k: &str| p.get(k).and_then(Value::as_bool);
                    let d = FollowingPrefs::default();
                    out.following = FollowingPrefs {
                        hide_replies: b("hideReplies").unwrap_or(d.hide_replies),
                        hide_replies_by_unfollowed: b("hideRepliesByUnfollowed")
                            .unwrap_or(d.hide_replies_by_unfollowed),
                        min_reply_likes: p
                            .get("hideRepliesByLikeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        hide_reposts: b("hideReposts").unwrap_or(d.hide_reposts),
                        hide_quote_posts: b("hideQuotePosts").unwrap_or(d.hide_quote_posts),
                    };
                }
                POST_INTERACTION => {
                    out.interaction = Interaction {
                        replies: p
                            .get("threadgateAllowRules")
                            .and_then(Value::as_array)
                            .map(|a| ReplyRules::from_json(a)),
                        quotes: Interaction::from_records(
                            None,
                            Some(&serde_json::json!({
                                "embeddingRules": p.get("postgateEmbeddingRules").cloned().unwrap_or_default()
                            })),
                        )
                        .quotes,
                    };
                }
                VERIFICATION => {
                    out.hide_verification_badges = p
                        .get("hideBadges")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                }
                "app.bsky.actor.defs#adultContentPref" => {
                    out.adult_content_enabled =
                        p.get("enabled").and_then(Value::as_bool).unwrap_or(false);
                }
                "app.bsky.actor.defs#contentLabelPref" => {
                    let vis = match s("visibility").as_deref() {
                        Some("hide") => LabelVisibility::Hide,
                        Some("warn") => LabelVisibility::Warn,
                        _ => LabelVisibility::Ignore,
                    };
                    if let Some(label) = s("label") {
                        out.content_labels.push((label, s("labelerDid"), vis));
                    }
                }
                "app.bsky.actor.defs#mutedWordsPref" => {
                    for item in p
                        .get("items")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        let Some(value) = item.get("value").and_then(Value::as_str) else {
                            continue;
                        };
                        let targets = item
                            .get("targets")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                            .filter_map(|t| t.as_str().map(str::to_string))
                            .collect();
                        out.muted_words.push(MutedWord {
                            value: value.to_string(),
                            targets,
                        });
                    }
                }
                _ => {}
            }
        }
        if !out
            .saved_feeds
            .iter()
            .any(|f| f.source == FeedSource::Following)
        {
            out.saved_feeds.insert(
                0,
                SavedFeed {
                    source: FeedSource::Following,
                    pinned: true,
                },
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::model::*;
    use serde_json::json;

    #[test]
    fn preferences_always_include_following() {
        let p = Preferences::from_json(&[json!({
            "$type": "app.bsky.actor.defs#savedFeedsPrefV2",
            "items": [{"type": "feed", "value": "at://x/app.bsky.feed.generator/y", "pinned": true, "id": "1"}]
        })]);
        assert_eq!(p.saved_feeds[0].source, FeedSource::Following);
        assert_eq!(p.saved_feeds.len(), 2);
    }

    #[test]
    fn edits_keep_unknown_preferences() {
        let mut raw = vec![
            json!({"$type": "app.bsky.actor.defs#personalDetailsPref", "birthDate": "2000-01-01"}),
            json!({"$type": "app.bsky.actor.defs#savedFeedsPrefV2", "items": [
                {"id": "a", "type": "timeline", "value": "following", "pinned": true},
                {"id": "b", "type": "feed", "value": "at://x/app.bsky.feed.generator/1", "pinned": true},
                {"id": "c", "type": "feed", "value": "at://x/app.bsky.feed.generator/2", "pinned": false},
                {"id": "d", "type": "feed", "value": "at://x/app.bsky.feed.generator/3", "pinned": true},
            ]}),
        ];
        let g = |n: u8| FeedSource::Generator(format!("at://x/app.bsky.feed.generator/{n}").into());
        for edit in [
            PrefEdit::AdultContent(true),
            PrefEdit::Label("porn".into(), LabelVisibility::Warn),
            PrefEdit::MuteWord("#Spoilers ".into()),
            PrefEdit::HideVerificationBadges(true),
            PrefEdit::MuteWord("spoilers".into()),
            PrefEdit::MoveFeed(g(3), -1),
            PrefEdit::PinFeed(g(2), true),
            PrefEdit::RemoveFeed(g(1)),
            PrefEdit::SaveFeed(g(4), false),
        ] {
            edit.apply(&mut raw);
        }
        let p = Preferences::from_json(&raw);
        assert_eq!(raw[0]["birthDate"], "2000-01-01");
        assert!(p.adult_content_enabled);
        assert_eq!(
            p.content_labels,
            vec![("porn".into(), None, LabelVisibility::Warn)]
        );
        assert_eq!(p.muted_words.len(), 1);
        assert_eq!(p.muted_words[0].value, "Spoilers");
        let order: Vec<(FeedSource, bool)> = p
            .saved_feeds
            .iter()
            .map(|f| (f.source.clone(), f.pinned))
            .collect();
        assert_eq!(
            order,
            vec![
                (FeedSource::Following, true),
                (g(3), true),
                (g(2), true),
                (g(4), false)
            ]
        );
        PrefEdit::UnmuteWord("Spoilers".into()).apply(&mut raw);
        assert!(Preferences::from_json(&raw).muted_words.is_empty());
    }
}
