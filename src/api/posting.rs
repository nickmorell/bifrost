//! Publishing posts: one, or a thread of them, each a reply to the one before.

use super::richtext::{self, DetectedKind};
use super::{
    AnyAgent, ApiError, Client, DraftVideo, LinkCard, POSTGATE, THREADGATE, conv, fetch_bytes,
    now_iso, parse_at_uri, strong_ref, xrpc_err,
};
use crate::model::{AtUri, Interaction, StrongRef, tid};
use serde_json::{Value, json};
use std::fmt::Debug;

/// Everything needed to publish a post.
#[derive(Debug, Clone, Default)]
pub struct PostDraft {
    pub text: String,
    /// (root, parent) of the post being replied to.
    pub reply: Option<(StrongRef, StrongRef)>,
    pub quote: Option<StrongRef>,
    pub images: Vec<DraftImage>,
    pub video: Option<DraftVideo>,
    pub link_card: Option<LinkCard>,
    /// Content warnings for the post's images, such as "sexual" or "graphic-media".
    pub labels: Vec<String>,
    pub langs: Vec<String>,
    /// Who can reply (for a new thread) and whether it can be quoted.
    pub interaction: Interaction,
}

#[derive(Clone)]
pub struct DraftImage {
    pub bytes: Vec<u8>,
    pub alt: String,
    pub width: u32,
    pub height: u32,
}

impl Debug for DraftImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DraftImage({} bytes, {}x{})",
            self.bytes.len(),
            self.width,
            self.height
        )
    }
}

impl Client {
    /// Links, mentions and tags in `text`, as facets. Mentions of handles that do not
    /// resolve are left as plain text, like the official app.
    pub(super) async fn facets(&self, text: &str) -> Vec<Value> {
        let mut facets = Vec::new();
        for found in richtext::detect(text) {
            let feature = match &found.kind {
                DetectedKind::Mention(handle) => match self.resolve_handle(handle).await {
                    Ok(did) => json!({"$type": "app.bsky.richtext.facet#mention", "did": did}),
                    Err(_) => continue,
                },
                DetectedKind::Link(url) => {
                    json!({"$type": "app.bsky.richtext.facet#link", "uri": url})
                }
                DetectedKind::Tag(tag) | DetectedKind::Cashtag(tag) => {
                    json!({"$type": "app.bsky.richtext.facet#tag", "tag": tag})
                }
            };
            facets.push(json!({
                "index": {"byteStart": found.start, "byteEnd": found.end},
                "features": [feature]
            }));
        }
        facets
    }

    /// Publishes a post and returns its URI.
    pub async fn create_post(&self, draft: PostDraft) -> Result<AtUri, ApiError> {
        let facets = self.facets(&draft.text).await;

        let media = if !draft.images.is_empty() {
            let mut images = Vec::new();
            for img in &draft.images {
                let blob = self.upload(img.bytes.clone()).await?;
                images.push(json!({
                    "image": blob, "alt": img.alt,
                    "aspectRatio": {"width": img.width.max(1), "height": img.height.max(1)}
                }));
            }
            Some(json!({"$type": "app.bsky.embed.images", "images": images}))
        } else if let Some(video) = &draft.video {
            let blob = self.upload_video(video).await?;
            let mut embed = json!({"$type": "app.bsky.embed.video", "video": blob});
            if !video.alt.trim().is_empty() {
                embed["alt"] = json!(video.alt.trim());
            }
            if let Some((w, h)) = video.info.size {
                embed["aspectRatio"] = json!({"width": w.max(1), "height": h.max(1)});
            }
            Some(embed)
        } else if let Some(card) = &draft.link_card {
            let mut external =
                json!({"uri": card.url, "title": card.title, "description": card.description});
            if let Some(img) = &card.image_url {
                // A missing thumbnail should not stop the post.
                if let Ok(bytes) = fetch_bytes(img).await
                    && let Ok(blob) = self.upload(bytes).await
                {
                    external["thumb"] = blob;
                }
            }
            Some(json!({"$type": "app.bsky.embed.external", "external": external}))
        } else {
            None
        };

        let embed = match (&draft.quote, media) {
            (Some(q), Some(media)) => Some(json!({
                "$type": "app.bsky.embed.recordWithMedia",
                "record": {"$type": "app.bsky.embed.record", "record": strong_ref(q)},
                "media": media
            })),
            (Some(q), None) => {
                Some(json!({"$type": "app.bsky.embed.record", "record": strong_ref(q)}))
            }
            (None, media) => media,
        };

        let mut record = json!({
            "$type": "app.bsky.feed.post",
            "text": draft.text,
            "createdAt": now_iso(),
            "langs": draft.langs,
        });
        if !draft.labels.is_empty() {
            let values: Vec<Value> = draft.labels.iter().map(|l| json!({ "val": l })).collect();
            record["labels"] =
                json!({"$type": "com.atproto.label.defs#selfLabels", "values": values});
        }
        if !facets.is_empty() {
            record["facets"] = Value::Array(facets);
        }
        if let Some(embed) = embed {
            record["embed"] = embed;
        }
        if let Some((root, parent)) = &draft.reply {
            record["reply"] = json!({"root": strong_ref(root), "parent": strong_ref(parent)});
        }
        // Replies follow the rules of the thread they are in, so only a new thread
        // gets a threadgate.
        let gate = draft
            .interaction
            .replies
            .as_ref()
            .filter(|_| draft.reply.is_none());
        if gate.is_none() && draft.interaction.quotes {
            return self.create_record("app.bsky.feed.post", record).await;
        }
        // The post and its gates are written together, so the post is never
        // briefly open to everyone.
        let repo = self.my_did().await?;
        let rkey = tid();
        let uri = AtUri::new(&repo, "app.bsky.feed.post", &rkey);
        let create = |collection: &str, value: Value| {
            json!({
                "$type": "com.atproto.repo.applyWrites#create",
                "collection": collection, "rkey": rkey, "value": value
            })
        };
        let mut writes = vec![create("app.bsky.feed.post", record)];
        if let Some(rules) = gate {
            writes.push(create(
                THREADGATE,
                json!({
                    "$type": THREADGATE, "post": uri, "allow": rules.to_json(),
                    "createdAt": now_iso()
                }),
            ));
        }
        if !draft.interaction.quotes {
            writes.push(create(
                POSTGATE,
                json!({
                    "$type": POSTGATE, "post": uri,
                    "embeddingRules": draft.interaction.embedding_rules(),
                    "createdAt": now_iso()
                }),
            ));
        }
        xrpc!(self, |a| a
            .api
            .com
            .atproto
            .repo
            .apply_writes(conv(json!({"repo": repo, "writes": writes}))?));
        Ok(uri)
    }

    /// Publishes `posts` as a thread, each replying to the one before, and returns
    /// the first one's URI. The first may itself be a reply; the rest keep its root.
    /// If one fails, those already sent stay and the error says how many went out.
    pub async fn create_thread(&self, posts: Vec<PostDraft>) -> Result<AtUri, ApiError> {
        let total = posts.len();
        let mut first: Option<AtUri> = None;
        let mut chain: Option<(StrongRef, StrongRef)> = None;
        for (i, mut post) in posts.into_iter().enumerate() {
            if let Some(reply) = chain.take() {
                post.reply = Some(reply);
            }
            let root = post.reply.as_ref().map(|r| r.0.clone());
            let sent = async {
                let uri = self.create_post(post).await?;
                let (_, _, rkey) = parse_at_uri(uri.as_str())
                    .ok_or_else(|| ApiError::Other(format!("not a post: {uri}")))?;
                let (_, cid) = self
                    .my_record("app.bsky.feed.post", &rkey)
                    .await?
                    .ok_or_else(|| ApiError::Other("the post just sent can't be found".into()))?;
                Ok::<_, ApiError>(StrongRef { uri, cid })
            }
            .await
            .map_err(|e| match i {
                0 => e,
                _ => ApiError::Other(format!(
                    "{i} of {total} posts went out, then: {}",
                    e.friendly()
                )),
            })?;
            first.get_or_insert_with(|| sent.uri.clone());
            chain = Some((root.unwrap_or_else(|| sent.clone()), sent));
        }
        first.ok_or_else(|| ApiError::Other("nothing to post".into()))
    }
}
