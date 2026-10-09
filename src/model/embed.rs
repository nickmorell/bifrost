//! What a post can embed: images, video, link cards and other records.

use super::article::{DOCUMENT, PUBLICATION};
use super::post::lenient_record;
use super::{AtUri, Author, Label, PostRecord, StrongRef};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AspectRatio {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageView {
    pub thumb: String,
    pub fullsize: String,
    #[serde(default)]
    pub alt: String,
    pub aspect_ratio: Option<AspectRatio>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalView {
    pub uri: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub thumb: Option<String>,
    /// Minutes to read, for an article.
    pub reading_time: Option<u32>,
    /// The publication an article comes from, such as a Standard.site blog.
    pub source: Option<ExternalSource>,
    /// The records behind the link: for an article, its document and publication.
    #[serde(default)]
    pub associated_refs: Vec<StrongRef>,
}

impl ExternalView {
    /// The article record behind the link, when Bluesky knows of one.
    pub fn document(&self) -> Option<&AtUri> {
        self.refs_in(DOCUMENT)
    }

    /// The publication record behind the link, to subscribe to.
    pub fn publication(&self) -> Option<&AtUri> {
        self.refs_in(PUBLICATION)
    }

    fn refs_in(&self, collection: &str) -> Option<&AtUri> {
        self.associated_refs
            .iter()
            .map(|r| &r.uri)
            .find(|u| u.collection() == Some(collection))
    }
}

/// Where an article comes from: its publication's name, address and colours.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSource {
    /// The publication's web address.
    pub uri: String,
    pub title: String,
    pub icon: Option<String>,
    pub theme: Option<SourceTheme>,
}

/// A publication's colours. Its button uses `accent` behind `accent_foreground`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTheme {
    pub accent: Option<Rgb>,
    pub accent_foreground: Option<Rgb>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "$type")]
pub enum Embed {
    #[serde(rename = "app.bsky.embed.images#view")]
    Images { images: Vec<ImageView> },
    #[serde(rename = "app.bsky.embed.video#view", rename_all = "camelCase")]
    Video {
        playlist: String,
        thumbnail: Option<String>,
        alt: Option<String>,
        aspect_ratio: Option<AspectRatio>,
    },
    #[serde(rename = "app.bsky.embed.external#view")]
    External { external: ExternalView },
    #[serde(rename = "app.bsky.embed.record#view")]
    Record { record: EmbeddedRecord },
    #[serde(rename = "app.bsky.embed.recordWithMedia#view")]
    RecordWithMedia {
        record: RecordWrapper,
        media: Box<Embed>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordWrapper {
    pub record: EmbeddedRecord,
}

/// A quoted post, or something else that can be embedded as a record.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "$type")]
pub enum EmbeddedRecord {
    #[serde(rename = "app.bsky.embed.record#viewRecord")]
    Post(Box<QuotedPost>),
    #[serde(rename = "app.bsky.embed.record#viewNotFound")]
    NotFound,
    #[serde(rename = "app.bsky.embed.record#viewBlocked")]
    Blocked,
    #[serde(rename = "app.bsky.embed.record#viewDetached")]
    Detached,
    #[serde(rename = "app.bsky.feed.defs#generatorView", rename_all = "camelCase")]
    FeedGenerator {
        uri: AtUri,
        display_name: String,
        creator: Author,
    },
    #[serde(rename = "app.bsky.graph.defs#listView")]
    List {
        uri: AtUri,
        name: String,
        creator: Author,
    },
    #[serde(rename = "app.bsky.graph.defs#starterPackViewBasic")]
    StarterPack {
        uri: AtUri,
        record: NamedRecord,
        creator: Author,
    },
    #[serde(other)]
    Other,
}

/// Just the name from a record, such as an embedded starter pack's.
#[derive(Debug, Clone, Deserialize)]
pub struct NamedRecord {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotedPost {
    pub uri: AtUri,
    pub cid: String,
    pub author: Author,
    #[serde(deserialize_with = "lenient_record")]
    pub value: PostRecord,
    #[serde(default)]
    pub embeds: Vec<Embed>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub indexed_at: String,
}
