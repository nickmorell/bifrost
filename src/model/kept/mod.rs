//! Posts kept on this computer, so they can be searched and read later without the
//! network, and the search language for them.

use super::{AtUri, Did, Embed, FacetFeature, PostView};
use serde::{Deserialize, Serialize};

mod links;
mod stats;
pub use links::{TopLink, top_links};
pub use stats::{STATS_DAYS, Stats};

/// A post as BiFrost keeps it: enough to find it, show it and open it again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kept {
    pub uri: AtUri,
    pub did: Did,
    pub handle: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
    pub text: String,
    /// When it was posted (RFC 3339).
    pub created: String,
    /// When it was first loaded here (RFC 3339).
    pub seen: String,
    /// Every web address in it: links in the text and the link card's.
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub card: Option<KeptLink>,
    /// Its pictures' small versions and descriptions.
    #[serde(default)]
    pub images: Vec<(String, String)>,
    #[serde(default)]
    pub liked: bool,
    #[serde(default)]
    pub saved: bool,
    /// Posted by the account it is kept for.
    #[serde(default)]
    pub mine: bool,
    /// A reply to another post.
    #[serde(default)]
    pub reply: bool,
    #[serde(default)]
    pub likes: u64,
    #[serde(default)]
    pub reposts: u64,
    #[serde(default)]
    pub replies: u64,
}

/// A link card as kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptLink {
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub thumb: Option<String>,
}

impl Kept {
    /// A post as seen by `me` at `now`.
    pub fn of(p: &PostView, me: &str, now: &str) -> Self {
        let mut links: Vec<String> = p
            .record
            .facets
            .iter()
            .flat_map(|f| &f.features)
            .filter_map(|f| match f {
                FacetFeature::Link { uri } => Some(uri.clone()),
                _ => None,
            })
            .collect();
        let (mut card, mut images) = (None, Vec::new());
        let mut embeds: Vec<&Embed> = p.embed.iter().collect();
        while let Some(e) = embeds.pop() {
            match e {
                Embed::External { external } => {
                    links.push(external.uri.clone());
                    card = Some(KeptLink {
                        url: external.uri.clone(),
                        title: external.title.clone(),
                        description: external.description.clone(),
                        thumb: external.thumb.clone(),
                    });
                }
                Embed::Images { images: list } => {
                    images.extend(list.iter().map(|i| (i.thumb.clone(), i.alt.clone())));
                }
                Embed::RecordWithMedia { media, .. } => embeds.push(media),
                _ => {}
            }
        }
        links.dedup();
        Self {
            uri: p.uri.clone(),
            did: p.author.did.clone(),
            handle: p.author.handle.clone(),
            name: p.author.display_name.clone().unwrap_or_default(),
            avatar: p.author.avatar.clone(),
            text: p.record.text.clone(),
            created: p.record.created_at.clone(),
            seen: now.to_string(),
            links,
            card,
            images,
            liked: p.viewer.like.is_some(),
            saved: p.viewer.bookmarked,
            mine: p.author.did == me,
            reply: p.record.reply.is_some(),
            likes: p.like_count,
            reposts: p.repost_count,
            replies: p.reply_count,
        }
    }

    /// Takes in a newer copy of the same post, keeping when it was first seen.
    pub fn refresh(&mut self, newer: Kept) {
        let seen = std::mem::take(&mut self.seen);
        *self = Self { seen, ..newer };
    }
}

/// A search over kept posts: words and quoted phrases that must all appear, and
/// `from:`, `since:`, `until:`, `domain:` and `is:liked` / `is:saved` / `is:mine`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub words: Vec<String>,
    pub from: Option<String>,
    /// `YYYY-MM-DD`, inclusive.
    pub since: Option<String>,
    pub until: Option<String>,
    pub domain: Option<String>,
    pub liked: bool,
    pub saved: bool,
    pub mine: bool,
}

impl Query {
    pub fn parse(q: &str) -> Self {
        let mut out = Self::default();
        let mut rest = q.trim();
        while !rest.is_empty() {
            let (token, after) = if let Some(quoted) = rest.strip_prefix('"') {
                let end = quoted.find('"').unwrap_or(quoted.len());
                (&quoted[..end], quoted.get(end + 1..).unwrap_or(""))
            } else {
                rest.split_once(char::is_whitespace).unwrap_or((rest, ""))
            };
            rest = after.trim_start();
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let lower = token.to_lowercase();
            let day = |d: &str| {
                chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                    .ok()
                    .map(|d| d.to_string())
            };
            match lower.split_once(':') {
                Some(("from", who)) if !who.is_empty() => {
                    out.from = Some(who.trim_start_matches('@').to_string());
                }
                Some(("since", d)) if day(d).is_some() => out.since = day(d),
                Some(("until", d)) if day(d).is_some() => out.until = day(d),
                Some(("domain", d)) if !d.is_empty() => {
                    out.domain = Some(d.trim_start_matches("www.").to_string());
                }
                Some(("is", "liked")) => out.liked = true,
                Some(("is", "saved" | "bookmarked")) => out.saved = true,
                Some(("is", "mine")) => out.mine = true,
                _ => out.words.push(lower),
            }
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn matches(&self, k: &Kept) -> bool {
        if (self.liked && !k.liked) || (self.saved && !k.saved) || (self.mine && !k.mine) {
            return false;
        }
        if let Some(who) = &self.from
            && !(k.handle.eq_ignore_ascii_case(who)
                || k.handle.to_lowercase().starts_with(&format!("{who}."))
                || k.did.as_str() == who)
        {
            return false;
        }
        let day = k.created.get(..10).unwrap_or("");
        if self.since.as_deref().is_some_and(|s| day < s)
            || self.until.as_deref().is_some_and(|u| day > u)
        {
            return false;
        }
        if let Some(domain) = &self.domain
            && !k.links.iter().any(|l| on_domain(l, domain))
        {
            return false;
        }
        if self.words.is_empty() {
            return true;
        }
        let mut hay = format!("{}\n{}\n{}", k.text, k.name, k.handle);
        if let Some(c) = &k.card {
            hay.push('\n');
            hay.push_str(&c.title);
            hay.push('\n');
            hay.push_str(&c.description);
        }
        for (_, alt) in &k.images {
            hay.push('\n');
            hay.push_str(alt);
        }
        let hay = hay.to_lowercase();
        self.words.iter().all(|w| hay.contains(w.as_str()))
    }
}

/// The site a web address is on, without `www.`.
pub fn domain_of(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    let host = host.split(':').next()?.to_lowercase();
    (!host.is_empty()).then(|| host.trim_start_matches("www.").to_string())
}

/// `url` is on `domain` or one of its subdomains.
fn on_domain(url: &str, domain: &str) -> bool {
    domain_of(url).is_some_and(|h| h == domain || h.ends_with(&format!(".{domain}")))
}

#[cfg(test)]
mod tests;
