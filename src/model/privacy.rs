//! Who your posts reach beyond the people you talk with: Bluesky's Discover and
//! other algorithmic feeds, and visitors who are not signed in. Bluesky has no
//! private accounts; these are requests that its own app honours.

use serde_json::{Map, Value, json};

/// The record that asks to be left out of algorithmic recommendations.
pub const DECLARATION: &str = "app.bsky.actor.contentVisibilityDeclaration";
/// The self-label on your profile that asks to be hidden from logged-out visitors.
pub const NO_UNAUTHENTICATED: &str = "!no-unauthenticated";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Privacy {
    /// Keep your posts out of Discover and other algorithmic feeds.
    pub hide_from_discover: bool,
    /// Hide your profile and posts from people who aren't signed in.
    pub hide_logged_out: bool,
}

impl Privacy {
    /// Read from your declaration and profile records; a missing record means no.
    pub fn from_records(declaration: Option<&Value>, profile: Option<&Value>) -> Self {
        Self {
            hide_from_discover: declaration
                .and_then(|d| d.get("hideFromAlgorithmicRecommendations"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            hide_logged_out: profile
                .and_then(|p| p.pointer("/labels/values"))
                .and_then(Value::as_array)
                .is_some_and(|v| {
                    v.iter()
                        .any(|l| l.get("val").and_then(Value::as_str) == Some(NO_UNAUTHENTICATED))
                }),
        }
    }

    /// The declaration record for `hide`.
    pub fn declaration(hide: bool) -> Value {
        json!({"$type": DECLARATION, "hideFromAlgorithmicRecommendations": hide})
    }
}

/// Adds or removes the logged-out label on a profile record, keeping its other
/// self-labels.
pub fn set_logged_out(profile: &mut Map<String, Value>, hide: bool) {
    let mut values: Vec<Value> = profile
        .get("labels")
        .and_then(|l| l.get("values"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    values.retain(|l| l.get("val").and_then(Value::as_str) != Some(NO_UNAUTHENTICATED));
    if hide {
        values.push(json!({"val": NO_UNAUTHENTICATED}));
    }
    if values.is_empty() {
        profile.remove("labels");
    } else {
        profile.insert(
            "labels".into(),
            json!({"$type": "com.atproto.label.defs#selfLabels", "values": values}),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_settings_and_treats_missing_as_no() {
        assert_eq!(Privacy::from_records(None, None), Privacy::default());
        let profile = json!({"labels": {"values": [{"val": "porn"}, {"val": NO_UNAUTHENTICATED}]}});
        let p = Privacy::from_records(Some(&Privacy::declaration(true)), Some(&profile));
        assert!(p.hide_from_discover && p.hide_logged_out);
    }

    #[test]
    fn the_logged_out_label_keeps_other_labels() {
        let mut profile = json!({"displayName": "Me", "labels": {"values": [{"val": "porn"}]}});
        let map = profile.as_object_mut().unwrap();
        set_logged_out(map, true);
        assert!(Privacy::from_records(None, Some(&Value::Object(map.clone()))).hide_logged_out);
        set_logged_out(map, false);
        assert_eq!(map["labels"]["values"], json!([{"val": "porn"}]));
        let mut bare = Map::new();
        set_logged_out(&mut bare, false);
        assert!(bare.get("labels").is_none());
    }
}
