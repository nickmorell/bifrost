//! The links most shared in kept posts lately: a "top links today" built only from
//! what this computer has seen.

use super::{Kept, domain_of};
use chrono::{DateTime, Utc};
use std::collections::{BTreeSet, HashMap};

/// A link and how many people shared it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopLink {
    pub url: String,
    /// The link card's title, when a post had one.
    pub title: String,
    pub domain: String,
    /// Different accounts that posted it.
    pub people: usize,
    pub posts: usize,
}

/// Sites whose links point back into Bluesky itself.
const SKIPPED: [&str; 2] = ["bsky.app", "go.bsky.app"];

/// Links in posts made after `since` that at least two people shared, most shared
/// first, at most `limit`.
pub fn top_links(posts: &[Kept], since: DateTime<Utc>, limit: usize) -> Vec<TopLink> {
    let mut found: HashMap<String, (TopLink, BTreeSet<&str>)> = HashMap::new();
    for k in posts {
        let recent = DateTime::parse_from_rfc3339(&k.created).is_ok_and(|t| t >= since);
        if !recent {
            continue;
        }
        let mut seen_here = BTreeSet::new();
        for url in &k.links {
            let Some(domain) = domain_of(url) else {
                continue;
            };
            let key = normal(url);
            if SKIPPED.contains(&domain.as_str()) || !seen_here.insert(key.clone()) {
                continue;
            }
            let card = k.card.as_ref().filter(|c| normal(&c.url) == key);
            let (link, people) = found.entry(key).or_insert_with(|| {
                (
                    TopLink {
                        url: url.clone(),
                        title: String::new(),
                        domain,
                        people: 0,
                        posts: 0,
                    },
                    BTreeSet::new(),
                )
            });
            if let Some(c) = card.filter(|_| link.title.is_empty()) {
                link.title.clone_from(&c.title);
            }
            link.posts += 1;
            people.insert(k.did.as_str());
        }
    }
    let mut out: Vec<TopLink> = found
        .into_values()
        .map(|(mut link, people)| {
            link.people = people.len();
            link
        })
        .filter(|l| l.people >= 2)
        .collect();
    out.sort_by(|a, b| {
        (b.people, b.posts)
            .cmp(&(a.people, a.posts))
            .then_with(|| a.url.cmp(&b.url))
    });
    out.truncate(limit);
    out
}

/// The address with tracking parameters, the fragment and a trailing slash left
/// out, so the same page shared two ways counts once.
fn normal(url: &str) -> String {
    let url = url.split('#').next().unwrap_or(url);
    let (base, query) = url.split_once('?').unwrap_or((url, ""));
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| !p.is_empty() && !p.starts_with("utm_") && !p.starts_with("ref="))
        .collect();
    let base = base.trim_end_matches('/').to_lowercase();
    let base = base
        .replacen("://www.", "://", 1)
        .replacen("http://", "https://", 1);
    if kept.is_empty() {
        base
    } else {
        format!("{base}?{}", kept.join("&"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::KeptLink;

    fn kept(did: &str, created: &str, links: &[&str]) -> Kept {
        Kept {
            uri: format!("at://{did}/app.bsky.feed.post/{created}").into(),
            did: did.into(),
            handle: "h.test".into(),
            name: String::new(),
            avatar: None,
            text: String::new(),
            created: created.into(),
            seen: created.into(),
            links: links.iter().map(ToString::to_string).collect(),
            card: None,
            images: Vec::new(),
            liked: false,
            saved: false,
            mine: false,
            reply: false,
            likes: 0,
            reposts: 0,
            replies: 0,
        }
    }

    #[test]
    fn counts_people_not_posts_and_only_recent_ones() {
        let since = "2026-10-07T12:00:00Z".parse().unwrap();
        let mut card = kept(
            "did:plc:a",
            "2026-10-08T09:00:00Z",
            &["https://news.test/story?utm_source=x"],
        );
        card.card = Some(KeptLink {
            url: "https://news.test/story".into(),
            title: "The story".into(),
            description: String::new(),
            thumb: Some("t".into()),
        });
        let posts = [
            card,
            kept(
                "did:plc:b",
                "2026-10-08T10:00:00Z",
                &["https://www.news.test/story/"],
            ),
            kept(
                "did:plc:b",
                "2026-10-08T11:00:00Z",
                &["https://news.test/story"],
            ),
            kept(
                "did:plc:a",
                "2026-10-08T09:00:00Z",
                &["https://once.test/x"],
            ),
            kept(
                "did:plc:b",
                "2026-10-08T09:00:00Z",
                &["https://once.test/x"],
            ),
            kept(
                "did:plc:c",
                "2026-10-08T09:00:00Z",
                &["https://once.test/x"],
            ),
            // Too old, and Bluesky's own links.
            kept(
                "did:plc:d",
                "2026-10-06T09:00:00Z",
                &["https://news.test/story"],
            ),
            kept(
                "did:plc:a",
                "2026-10-08T09:00:00Z",
                &["https://bsky.app/profile/x"],
            ),
            kept(
                "did:plc:b",
                "2026-10-08T09:00:00Z",
                &["https://bsky.app/profile/x"],
            ),
            kept(
                "did:plc:z",
                "2026-10-08T09:00:00Z",
                &["https://lonely.test"],
            ),
        ];
        let top = top_links(&posts, since, 10);
        let got: Vec<(&str, usize, usize)> = top
            .iter()
            .map(|l| (l.domain.as_str(), l.people, l.posts))
            .collect();
        assert_eq!(got, [("once.test", 3, 3), ("news.test", 2, 3)]);
        assert_eq!(top[1].title, "The story");
        assert_eq!(top_links(&posts, since, 1).len(), 1);
    }
}
