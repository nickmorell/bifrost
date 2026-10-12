//! Posts: the record, its rich-text facets and the post as the app view returns it.

use super::{AtUri, Author, Did, Embed, Label, Mine, ReplyRef, ThreadgateView};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByteSlice {
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "$type")]
pub enum FacetFeature {
    #[serde(rename = "app.bsky.richtext.facet#mention")]
    Mention { did: Did },
    #[serde(rename = "app.bsky.richtext.facet#link")]
    Link { uri: String },
    #[serde(rename = "app.bsky.richtext.facet#tag")]
    Tag { tag: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Facet {
    pub index: ByteSlice,
    #[serde(default)]
    pub features: Vec<FacetFeature>,
}

/// The `app.bsky.feed.post` record.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostRecord {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub facets: Vec<Facet>,
    #[serde(default)]
    pub created_at: String,
    pub reply: Option<ReplyRef>,
    #[serde(default)]
    pub langs: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostViewer {
    pub like: Option<Mine>,
    pub repost: Option<Mine>,
    #[serde(default)]
    pub reply_disabled: bool,
    #[serde(default)]
    pub embedding_disabled: bool,
    #[serde(default)]
    pub bookmarked: bool,
    #[serde(default)]
    pub thread_muted: bool,
    /// Pinned to its author's profile.
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostView {
    pub uri: AtUri,
    pub cid: String,
    pub author: Author,
    #[serde(deserialize_with = "lenient_record")]
    pub record: PostRecord,
    pub embed: Option<Embed>,
    #[serde(default)]
    pub reply_count: u64,
    #[serde(default)]
    pub repost_count: u64,
    #[serde(default)]
    pub like_count: u64,
    #[serde(default)]
    pub quote_count: u64,
    #[serde(default)]
    pub indexed_at: String,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub viewer: PostViewer,
    /// Who may reply, when the thread's author limited it.
    pub threadgate: Option<ThreadgateView>,
}

impl PostView {
    /// The web links in the post: those in its text, then its link card's, each once.
    pub fn links(&self) -> Vec<&str> {
        let in_text = self.record.facets.iter().flat_map(|f| &f.features);
        let mut out: Vec<&str> = in_text
            .filter_map(|f| match f {
                FacetFeature::Link { uri } => Some(uri.as_str()),
                _ => None,
            })
            .collect();
        let card = match &self.embed {
            Some(Embed::External { external }) => Some(external),
            Some(Embed::RecordWithMedia { media, .. }) => match media.as_ref() {
                Embed::External { external } => Some(external),
                _ => None,
            },
            _ => None,
        };
        out.extend(card.map(|c| c.uri.as_str()));
        let mut seen = std::collections::HashSet::new();
        out.retain(|u| seen.insert(*u));
        out
    }
}

/// A post or quoted record whose `value` is not a post deserializes to an empty
/// record rather than failing the surrounding page.
pub(super) fn lenient_record<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<PostRecord, D::Error> {
    let v = Value::deserialize(d)?;
    Ok(serde_json::from_value(v).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use crate::model::*;
    use serde_json::json;

    #[test]
    fn links_in_text_and_card_each_once() {
        let link = |uri: &str| {
            json!({"index": {"byteStart": 0, "byteEnd": 1},
            "features": [{"$type": "app.bsky.richtext.facet#link", "uri": uri}]})
        };
        let post: PostView = serde_json::from_value(json!({
            "uri": "at://did:plc:a/app.bsky.feed.post/1",
            "cid": "bafy",
            "author": {"did": "did:plc:a", "handle": "a.test"},
            "record": {"text": "x", "facets": [link("https://a.test/1"), link("https://b.test/2"), link("https://a.test/1")]},
            "embed": {"$type": "app.bsky.embed.external#view",
                "external": {"uri": "https://b.test/2", "title": "", "description": ""}},
        }))
        .unwrap();
        assert_eq!(post.links(), ["https://a.test/1", "https://b.test/2"]);
    }

    #[test]
    fn unknown_embed_does_not_fail_post() {
        let post: PostView = serde_json::from_value(json!({
            "uri": "at://did:plc:a/app.bsky.feed.post/1",
            "cid": "bafy",
            "author": {"did": "did:plc:a", "handle": "a.test"},
            "record": {"$type": "app.bsky.feed.post", "text": "hi", "createdAt": "2026-01-01T00:00:00Z"},
            "embed": {"$type": "app.bsky.embed.somethingNew#view", "x": 1},
            "indexedAt": "2026-01-01T00:00:00Z"
        }))
        .unwrap();
        assert!(matches!(post.embed, Some(Embed::Other)));
        assert_eq!(post.record.text, "hi");
    }

    #[test]
    fn parses_facets() {
        let rec: PostRecord = serde_json::from_value(json!({
            "text": "hi @a.test",
            "facets": [{"index": {"byteStart": 3, "byteEnd": 10},
                        "features": [{"$type": "app.bsky.richtext.facet#mention", "did": "did:plc:a"}]}]
        }))
        .unwrap();
        assert!(
            matches!(&rec.facets[0].features[0], FacetFeature::Mention { did } if did == "did:plc:a")
        );
    }
}
