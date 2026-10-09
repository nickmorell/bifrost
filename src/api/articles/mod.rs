//! Long-form articles: reading Standard.site documents and WhiteWind entries from
//! their authors' servers, and your Standard.site subscriptions.

mod repo;

use super::{ApiError, Client, now_iso};
use crate::model::{
    Article, ArticleRef, ArticleSummary, AtUri, Block, DOCUMENT, PUBLICATION, Publication,
    SUBSCRIPTION, WHITEWIND, blocks,
};
use crate::net::{Transport, shared};
use repo::{Pds, Record};
use serde_json::{Value, json};

/// How many recent articles each subscribed publication contributes.
const PER_PUBLICATION: u32 = 20;
/// How many of an author's newest documents are searched for an article that moved.
const MOVED_SEARCH: u32 = 100;
/// How many articles the Subscriptions list keeps.
const MAX_ARTICLES: usize = 100;

/// Reads articles over any transport, so tests can use a fake.
struct Reader<'a>(&'a dyn Transport);

impl Reader<'_> {
    /// A Standard.site document. If its record isn't where `uri` says, which happens
    /// when a blog publishes an article again, the one at the `link` it was shared by
    /// is looked for among the author's documents.
    async fn standard(&self, uri: &AtUri, link: Option<&str>) -> Result<Article, ApiError> {
        let (did, rkey) = parts(uri)?;
        let pds = Pds::of(self.0, did).await?;
        let doc = match pds.record(did, DOCUMENT, rkey).await {
            Ok(r) => r.value,
            Err(e) => {
                let path = link.and_then(web_path).ok_or_else(|| e.clone())?;
                let docs = pds.records(did, DOCUMENT, MOVED_SEARCH).await?;
                docs.into_iter()
                    .find(|d| d.value["path"].as_str().and_then(web_path_of) == Some(path))
                    .map(|d| d.value)
                    .ok_or(e)?
            }
        };
        let text = |key: &str| doc[key].as_str().unwrap_or_default().trim().to_string();
        let site = text("site");
        let publication = match AtUri::from(site.as_str()) {
            u if u.collection() == Some(PUBLICATION) => self.publication(&u).await.ok(),
            _ => None,
        };
        let home = publication
            .as_ref()
            .map_or(site.as_str(), |p| p.url.as_str());
        let url = Some(format!("{}{}", home.trim_end_matches('/'), text("path")))
            .filter(|u| u.starts_with("https://"));
        let mut body = blocks(&text("textContent"));
        if body.is_empty() {
            leaflet_blocks(&doc["content"], &mut body);
        }
        if body.is_empty() && !text("description").is_empty() {
            body.push(Block::Paragraph(text("description")));
        }
        Ok(Article {
            title: text("title"),
            description: text("description"),
            published: text("publishedAt"),
            url,
            cover: pds.blob_url(did, &doc["coverImage"]),
            blocks: body,
            publication,
        })
    }

    async fn whitewind(&self, did: &str, rkey: &str, author: &str) -> Result<Article, ApiError> {
        let pds = Pds::of(self.0, did).await?;
        let entry = pds.record(did, WHITEWIND, rkey).await?.value;
        let text = |key: &str| entry[key].as_str().unwrap_or_default().trim().to_string();
        Ok(Article {
            title: text("title"),
            description: text("subtitle"),
            published: text("createdAt"),
            url: Some(format!("https://whtwnd.com/{author}/{rkey}")),
            cover: None,
            blocks: blocks(&text("content")),
            publication: None,
        })
    }

    async fn publication(&self, uri: &AtUri) -> Result<Publication, ApiError> {
        let (did, rkey) = parts(uri)?;
        let pds = Pds::of(self.0, did).await?;
        let v = pds.record(did, PUBLICATION, rkey).await?.value;
        Ok(Publication {
            uri: uri.clone(),
            name: v["name"].as_str().unwrap_or_default().to_string(),
            url: v["url"].as_str().unwrap_or_default().to_string(),
            icon: pds.blob_url(did, &v["icon"]),
        })
    }

    /// Your subscriptions: each record and the publication it follows.
    async fn subscriptions(&self, me: &str) -> Result<Vec<(AtUri, AtUri)>, ApiError> {
        let pds = Pds::of(self.0, me).await?;
        let records = pds.records(me, SUBSCRIPTION, 100).await?;
        Ok(records
            .into_iter()
            .filter_map(|r| {
                let publication = AtUri::from(r.value["publication"].as_str()?);
                Some((AtUri::from(r.uri), publication))
            })
            .collect())
    }

    /// The newest articles from these publications, newest first.
    async fn latest(&self, publications: &[AtUri]) -> Vec<ArticleSummary> {
        let mut out = Vec::new();
        for uri in publications {
            let Ok(publication) = self.publication(uri).await else {
                continue;
            };
            let Some(did) = uri.did() else { continue };
            let Ok(pds) = Pds::of(self.0, did).await else {
                continue;
            };
            let docs = pds
                .records(did, DOCUMENT, PER_PUBLICATION)
                .await
                .unwrap_or_default();
            out.extend(
                docs.iter()
                    .filter(|d| d.value["site"].as_str() == Some(uri.as_str()))
                    .map(|d| summary(d, &publication.name)),
            );
        }
        out.sort_by(|a, b| b.published.cmp(&a.published));
        out.truncate(MAX_ARTICLES);
        out
    }
}

fn summary(d: &Record, publication: &str) -> ArticleSummary {
    let text = |key: &str| d.value[key].as_str().unwrap_or_default().trim().to_string();
    ArticleSummary {
        uri: AtUri::from(d.uri.as_str()),
        title: text("title"),
        description: text("description"),
        published: text("publishedAt"),
        publication: publication.to_string(),
    }
}

/// Leaflet's own content format, when a document has no plain text: its text and
/// heading blocks, in order.
fn leaflet_blocks(v: &Value, out: &mut Vec<Block>) {
    match v {
        Value::Object(map) => {
            if let Some(text) = map.get("plaintext").and_then(Value::as_str) {
                let text = text.trim().to_string();
                if !text.is_empty() {
                    let kind = map.get("$type").and_then(Value::as_str).unwrap_or("");
                    out.push(if kind.ends_with(".header") {
                        Block::Heading(text)
                    } else {
                        Block::Paragraph(text)
                    });
                }
                return;
            }
            map.values().for_each(|c| leaflet_blocks(c, out));
        }
        Value::Array(items) => items.iter().for_each(|c| leaflet_blocks(c, out)),
        _ => {}
    }
}

/// The path of a web address, without the query, fragment or slashes at either
/// end: `https://blog.example/posts/hi/?x=1` is `posts/hi`.
fn web_path(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let path = rest.find('/').map_or("", |i| &rest[i..]);
    web_path_of(path)
}

/// A document's `path` as [`web_path`] gives it.
fn web_path_of(path: &str) -> Option<&str> {
    let path = path.split(['?', '#']).next().unwrap_or_default();
    Some(path.trim_matches('/')).filter(|p| !p.is_empty())
}

fn parts(uri: &AtUri) -> Result<(&str, &str), ApiError> {
    uri.did()
        .zip(uri.rkey())
        .ok_or_else(|| ApiError::Other(format!("not a record: {uri}")))
}

impl Client {
    pub async fn article(&self, article: &ArticleRef) -> Result<Article, ApiError> {
        let reader = Reader(shared());
        match article {
            ArticleRef::Standard { uri, link } => reader.standard(uri, link.as_deref()).await,
            ArticleRef::WhiteWind { author, rkey } => {
                let did = if author.starts_with("did:") {
                    author.clone()
                } else {
                    self.resolve_handle(author).await?.to_string()
                };
                reader.whitewind(&did, rkey, author).await
            }
        }
    }

    /// Your Standard.site subscriptions: each record and its publication.
    pub async fn article_subscriptions(&self) -> Result<Vec<(AtUri, AtUri)>, ApiError> {
        let me = self.my_did().await?;
        Reader(shared()).subscriptions(&me).await
    }

    /// Subscribes to a publication; gives the subscription's record.
    pub async fn subscribe(&self, publication: &AtUri) -> Result<AtUri, ApiError> {
        self.create_record(
            SUBSCRIPTION,
            json!({"$type": SUBSCRIPTION, "publication": publication, "createdAt": now_iso()}),
        )
        .await
    }

    /// The newest articles from these publications.
    pub async fn latest_articles(&self, publications: &[AtUri]) -> Vec<ArticleSummary> {
        Reader(shared()).latest(publications).await
    }
}

#[cfg(test)]
mod tests;
