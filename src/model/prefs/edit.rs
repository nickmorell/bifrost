//! Changing preferences without losing what other apps stored.

use super::{
    ADULT, FeedSource, LABEL, LabelVisibility, MUTED_WORDS, POST_INTERACTION, SAVED_FEEDS,
    VERIFICATION,
};
use crate::model::{Interaction, tid};
use serde_json::Value;

/// One change to the account's preferences. Edits are applied to the latest copy
/// from the server, so they never undo a change made elsewhere in the meantime.
#[derive(Debug, Clone, PartialEq)]
pub enum PrefEdit {
    AdultContent(bool),
    /// Bluesky's own label (not one from another labeler) and what to do with it.
    Label(String, LabelVisibility),
    MuteWord(String),
    UnmuteWord(String),
    SaveFeed(FeedSource, bool),
    RemoveFeed(FeedSource),
    PinFeed(FeedSource, bool),
    /// Move a pinned feed one place earlier (-1) or later (1) among the pinned ones.
    MoveFeed(FeedSource, i32),
    /// The reply and quote settings new posts start with.
    PostInteraction(Interaction),
    /// Hide (`true`) or show the blue verification checks.
    HideVerificationBadges(bool),
}

impl PrefEdit {
    /// Applies the edit to a full preferences array, adding the preference if it is
    /// missing.
    pub fn apply(&self, prefs: &mut Vec<Value>) {
        fn find<'a>(prefs: &'a mut Vec<Value>, ty: &str, init: Value) -> &'a mut Value {
            let i = if let Some(i) = prefs
                .iter()
                .position(|p| p.get("$type").and_then(Value::as_str) == Some(ty))
            {
                i
            } else {
                let mut v = init;
                v["$type"] = Value::from(ty);
                prefs.push(v);
                prefs.len() - 1
            };
            &mut prefs[i]
        }
        fn items(p: &mut Value) -> &mut Vec<Value> {
            if !p.get("items").is_some_and(Value::is_array) {
                p["items"] = Value::Array(Vec::new());
            }
            #[expect(clippy::expect_used, reason = "set to an array on the line above")]
            p["items"].as_array_mut().expect("just set")
        }
        match self {
            PrefEdit::AdultContent(on) => {
                find(prefs, ADULT, serde_json::json!({}))["enabled"] = Value::from(*on);
            }
            PrefEdit::HideVerificationBadges(hide) => {
                find(prefs, VERIFICATION, serde_json::json!({}))["hideBadges"] = Value::from(*hide);
            }
            PrefEdit::Label(label, vis) => {
                let existing = prefs.iter_mut().find(|p| {
                    p.get("$type").and_then(Value::as_str) == Some(LABEL)
                        && p.get("label").and_then(Value::as_str) == Some(label)
                        && p.get("labelerDid").is_none()
                });
                match existing {
                    Some(p) => p["visibility"] = Value::from(vis.as_str()),
                    None => prefs.push(serde_json::json!({
                        "$type": LABEL, "label": label, "visibility": vis.as_str()
                    })),
                }
            }
            PrefEdit::MuteWord(word) => {
                let word = word.trim().trim_start_matches('#');
                if word.is_empty() {
                    return;
                }
                let list = items(find(prefs, MUTED_WORDS, serde_json::json!({})));
                let taken = list.iter().any(|w| {
                    w.get("value")
                        .and_then(Value::as_str)
                        .is_some_and(|v| v.eq_ignore_ascii_case(word))
                });
                if !taken {
                    list.push(serde_json::json!({
                        "id": tid(), "value": word, "targets": ["content", "tag"], "actorTarget": "all"
                    }));
                }
            }
            PrefEdit::UnmuteWord(word) => {
                items(find(prefs, MUTED_WORDS, serde_json::json!({})))
                    .retain(|w| w.get("value").and_then(Value::as_str) != Some(word));
            }
            PrefEdit::SaveFeed(source, pinned) => {
                let list = items(find(prefs, SAVED_FEEDS, serde_json::json!({})));
                if list.is_empty() && *source != FeedSource::Following {
                    list.push(serde_json::json!({
                        "id": tid(), "type": "timeline", "value": "following", "pinned": true
                    }));
                }
                if let Some(item) = list.iter_mut().find(|i| source.matches(i)) {
                    item["pinned"] = Value::from(*pinned);
                } else {
                    let (ty, value) = source.pref_item();
                    list.push(serde_json::json!({
                        "id": tid(), "type": ty, "value": value, "pinned": pinned
                    }));
                }
            }
            PrefEdit::RemoveFeed(source) => {
                items(find(prefs, SAVED_FEEDS, serde_json::json!({})))
                    .retain(|i| !source.matches(i));
            }
            PrefEdit::PinFeed(source, pinned) => {
                if let Some(item) = items(find(prefs, SAVED_FEEDS, serde_json::json!({})))
                    .iter_mut()
                    .find(|i| source.matches(i))
                {
                    item["pinned"] = Value::from(*pinned);
                }
            }
            PrefEdit::PostInteraction(i) => {
                let p = find(prefs, POST_INTERACTION, serde_json::json!({}));
                match &i.replies {
                    Some(r) => p["threadgateAllowRules"] = Value::Array(r.to_json()),
                    None => _ = p.as_object_mut().map(|o| o.remove("threadgateAllowRules")),
                }
                p["postgateEmbeddingRules"] = Value::Array(i.embedding_rules());
            }
            PrefEdit::MoveFeed(source, step) => {
                let list = items(find(prefs, SAVED_FEEDS, serde_json::json!({})));
                let pinned: Vec<usize> = (0..list.len())
                    .filter(|&i| list[i].get("pinned").and_then(Value::as_bool) == Some(true))
                    .collect();
                let Some(at) = pinned.iter().position(|&i| source.matches(&list[i])) else {
                    return;
                };
                let to = at as i64 + i64::from(*step);
                if to >= 0 && (to as usize) < pinned.len() {
                    list.swap(pinned[at], pinned[to as usize]);
                }
            }
        }
    }
}
