//! Deciding what to hide, blur or warn about.
//!
//! The official TypeScript SDK ships this logic as `moderatePost`; there is no Rust
//! equivalent, so this is a deliberately small version covering what the MVP needs:
//! Bluesky's own labels, the user's adult-content and label preferences, muted words,
//! and muted or blocked accounts.

use crate::model::{Author, Label, LabelVisibility, PostRecord, Preferences};

/// What the UI should do with a post.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decision {
    /// Collapse the whole post behind a notice. The string says why.
    pub hide: Option<String>,
    /// Keep the post but cover its images and video. The string says why.
    pub blur_media: Option<String>,
}

impl Decision {
    pub fn is_clear(&self) -> bool {
        self.hide.is_none() && self.blur_media.is_none()
    }
}

/// Where a post is being shown. Feeds are stricter than a thread the user opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Feed,
    Thread,
    Profile,
}

/// Label values that apply to media rather than the whole post.
const MEDIA_LABELS: [&str; 5] = ["porn", "sexual", "nudity", "graphic-media", "gore"];
const ADULT_LABELS: [&str; 3] = ["porn", "sexual", "nudity"];

fn default_visibility(label: &str) -> LabelVisibility {
    match label {
        "porn" => LabelVisibility::Hide,
        "sexual" | "graphic-media" | "gore" => LabelVisibility::Warn,
        _ => LabelVisibility::Ignore,
    }
}

fn label_name(label: &str) -> &str {
    match label {
        "porn" => "Adult content",
        "sexual" => "Sexually suggestive",
        "nudity" => "Non-sexual nudity",
        "graphic-media" | "gore" => "Graphic media",
        other => other,
    }
}

fn visibility(prefs: &Preferences, label: &str) -> LabelVisibility {
    if ADULT_LABELS.contains(&label) && !prefs.adult_content_enabled {
        return LabelVisibility::Hide;
    }
    label_setting(prefs, label)
}

/// The labels the moderation screen lets you set, with what they cover.
pub const FILTERS: [(&str, &str, &str); 4] = [
    ("porn", "Adult content", "Explicit sexual images."),
    ("sexual", "Sexually suggestive", "Does not include nudity."),
    (
        "graphic-media",
        "Graphic media",
        "Explicit or potentially disturbing media.",
    ),
    ("nudity", "Non-sexual nudity", "For example artistic nudes."),
];

/// Whether a label needs adult content turned on before it can be shown.
pub fn is_adult(label: &str) -> bool {
    ADULT_LABELS.contains(&label)
}

/// Your setting for one of Bluesky's labels, or the default when you have none.
pub fn label_setting(prefs: &Preferences, label: &str) -> LabelVisibility {
    prefs
        .content_labels
        .iter()
        .find(|(l, labeler, _)| l == label && labeler.is_none())
        .map_or_else(|| default_visibility(label), |(_, _, v)| v.clone())
}

/// Active (non-negated) label values.
fn active(labels: &[Label]) -> Vec<&str> {
    labels
        .iter()
        .filter(|l| {
            !l.neg
                && !labels
                    .iter()
                    .any(|n| n.neg && n.val == l.val && n.src == l.src)
        })
        .map(|l| l.val.as_str())
        .collect()
}

pub fn decide(
    prefs: &Preferences,
    author: &Author,
    record: &PostRecord,
    post_labels: &[Label],
    my_did: &str,
    ctx: Context,
) -> Decision {
    let mut d = Decision::default();
    let mine = author.did == my_did;

    if !mine {
        if author.viewer.blocking.is_some() || author.viewer.blocked_by {
            d.hide = Some("Blocked account".into());
            return d;
        }
        if author.viewer.muted && ctx != Context::Profile {
            d.hide = Some("Post from an account you muted".into());
            return d;
        }
        if let Some(word) = muted_word(prefs, record) {
            d.hide = Some(format!("Hidden by muted word \"{word}\""));
            return d;
        }
    }

    for val in active(post_labels)
        .into_iter()
        .chain(active(&author.labels))
    {
        match val {
            "!hide" => {
                d.hide = Some("Hidden by moderators".into());
                return d;
            }
            "!warn" => d.hide = d.hide.or(Some("Content warning".into())),
            v if MEDIA_LABELS.contains(&v) => match visibility(prefs, v) {
                // Hidden labels on your own posts still get shown behind a blur.
                LabelVisibility::Hide if !mine && ctx == Context::Feed => {
                    d.hide = Some(label_name(v).to_string());
                    return d;
                }
                LabelVisibility::Hide | LabelVisibility::Warn => {
                    d.blur_media = d.blur_media.or(Some(label_name(v).to_string()));
                }
                LabelVisibility::Ignore => {}
            },
            _ => {}
        }
    }
    d
}

/// The first muted word that matches the post, if any.
pub fn muted_word<'a>(prefs: &'a Preferences, record: &PostRecord) -> Option<&'a str> {
    let text = record.text.to_lowercase();
    let tags: Vec<String> = record
        .tags
        .iter()
        .cloned()
        .chain(record.facets.iter().flat_map(|f| {
            f.features.iter().filter_map(|feat| match feat {
                crate::model::FacetFeature::Tag { tag } => Some(tag.clone()),
                _ => None,
            })
        }))
        .map(|t| t.to_lowercase())
        .collect();

    prefs.muted_words.iter().find_map(|w| {
        let word = w.value.trim_start_matches('#').to_lowercase();
        if word.is_empty() {
            return None;
        }
        let in_tags = tags.contains(&word);
        let in_content = w.targets.iter().any(|t| t == "content") && contains_word(&text, &word);
        (in_tags || in_content).then_some(w.value.as_str())
    })
}

/// Whole-word match for single words, substring match for phrases.
fn contains_word(text: &str, word: &str) -> bool {
    if word.contains(char::is_whitespace) {
        return text.contains(word);
    }
    text.split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
        .any(|w| w == word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ActorViewer, MutedWord};

    fn author(did: &str) -> Author {
        Author {
            did: did.into(),
            handle: "a.test".into(),
            display_name: None,
            avatar: None,
            labels: vec![],
            viewer: ActorViewer::default(),
            status: None,
        }
    }

    fn label(val: &str) -> Label {
        Label {
            src: "did:plc:ar7c4by46qjdydhdevvrndac".into(),
            val: val.into(),
            neg: false,
        }
    }

    fn text(t: &str) -> PostRecord {
        PostRecord {
            text: t.into(),
            ..Default::default()
        }
    }

    #[test]
    fn porn_hidden_in_feed_when_adult_disabled() {
        let d = decide(
            &Preferences::default(),
            &author("did:x"),
            &text(""),
            &[label("porn")],
            "did:me",
            Context::Feed,
        );
        assert!(d.hide.is_some());
    }

    #[test]
    fn graphic_media_blurs() {
        let prefs = Preferences {
            adult_content_enabled: true,
            ..Default::default()
        };
        let d = decide(
            &prefs,
            &author("did:x"),
            &text(""),
            &[label("graphic-media")],
            "did:me",
            Context::Feed,
        );
        assert!(d.hide.is_none());
        assert!(d.blur_media.is_some());
    }

    #[test]
    fn negated_label_is_ignored() {
        let mut neg = label("!hide");
        neg.neg = true;
        let d = decide(
            &Preferences::default(),
            &author("did:x"),
            &text(""),
            &[label("!hide"), neg],
            "did:me",
            Context::Feed,
        );
        assert!(d.is_clear());
    }

    #[test]
    fn muted_words_match_whole_words() {
        let prefs = Preferences {
            muted_words: vec![MutedWord {
                value: "spoiler".into(),
                targets: vec!["content".into(), "tag".into()],
            }],
            ..Default::default()
        };
        let a = author("did:x");
        assert!(
            decide(
                &prefs,
                &a,
                &text("Big Spoiler ahead"),
                &[],
                "did:me",
                Context::Feed
            )
            .hide
            .is_some()
        );
        assert!(
            decide(
                &prefs,
                &a,
                &text("unspoilered"),
                &[],
                "did:me",
                Context::Feed
            )
            .is_clear()
        );
    }

    #[test]
    fn muted_account_hidden_in_feed_not_profile() {
        let mut a = author("did:x");
        a.viewer.muted = true;
        assert!(
            decide(
                &Preferences::default(),
                &a,
                &text(""),
                &[],
                "did:me",
                Context::Feed
            )
            .hide
            .is_some()
        );
        assert!(
            decide(
                &Preferences::default(),
                &a,
                &text(""),
                &[],
                "did:me",
                Context::Profile
            )
            .is_clear()
        );
    }
}
