//! ATstore (atstore.fyi), the directory of apps and services on the AT Protocol: its
//! listings, a listing's own page, and the catalog BiFrost keeps of them. The types
//! read ATstore's JSON as it comes and are kept on this computer in the same shape.

use super::AtUri;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One app or service in the directory (`fyi.atstore.listing.detail`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub uri: AtUri,
    pub name: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub description: String,
    pub icon_url: Option<String>,
    /// A wide picture, 16:9.
    pub hero_image_url: Option<String>,
    /// The app it belongs under, such as "Bluesky Feed Generators" or its own name.
    #[serde(default)]
    pub category: String,
    /// What it is about, in lower case, such as "social" or "bluesky client".
    #[serde(default)]
    pub app_tags: Vec<String>,
    /// The average of its reviews out of 5, as ATstore words it ("4.8").
    pub rating: Option<String>,
    #[serde(default)]
    pub review_count: u64,
    /// The Bluesky handle of the app's own account.
    pub product_account_handle: Option<String>,
}

impl Listing {
    /// Its rating out of 5, when it has reviews.
    pub fn stars(&self) -> Option<f32> {
        if self.review_count == 0 {
            return None;
        }
        self.rating.as_deref()?.trim().parse().ok()
    }

    /// Whether every word of `query` appears in its name, tagline, description,
    /// category, tags or handle, ignoring case. An empty query matches everything.
    pub fn matches(&self, query: &str) -> bool {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return true;
        }
        let mut haystack = [
            self.name.as_str(),
            &self.tagline,
            &self.description,
            &self.category,
            self.product_account_handle.as_deref().unwrap_or_default(),
        ]
        .join("\n");
        for t in &self.app_tags {
            haystack.push('\n');
            haystack.push_str(t);
        }
        let haystack = haystack.to_lowercase();
        words.iter().all(|w| haystack.contains(w.as_str()))
    }
}

/// A listing's own page: the listing, and what only its page has.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingDetail {
    pub listing: Listing,
    #[serde(default)]
    pub screenshots: Vec<String>,
    /// Where the app itself lives on the web.
    pub external_url: Option<String>,
    /// More of its pages: help, privacy policy, terms and so on.
    #[serde(default)]
    pub links: Vec<ListingLink>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListingLink {
    pub uri: String,
}

/// The directory as BiFrost last read it, kept so it opens at once and offline.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    /// When it was read, as an RFC 3339 time; empty when it never was.
    pub fetched: String,
    /// Every listing, the most popular first.
    pub listings: Vec<Listing>,
    /// The listing pages opened so far, by listing.
    #[serde(default)]
    pub details: BTreeMap<AtUri, ListingDetail>,
}

impl Catalog {
    /// Whether it was read longer than `hours` before `now` (RFC 3339), or never.
    pub fn older_than(&self, hours: i64, now: &str) -> bool {
        let parse = chrono::DateTime::parse_from_rfc3339;
        match (parse(&self.fetched), parse(now)) {
            (Ok(then), Ok(now)) => now - then > chrono::Duration::hours(hours),
            _ => true,
        }
    }

    pub fn listing(&self, uri: &AtUri) -> Option<&Listing> {
        self.listings.iter().find(|l| &l.uri == uri)
    }

    /// Takes the listings of a newer read, keeping the pages of listings still there.
    pub fn replace(&mut self, newer: Catalog) {
        let mut details = std::mem::take(&mut self.details);
        details.retain(|uri, _| newer.listings.iter().any(|l| &l.uri == uri));
        *self = Catalog { details, ..newer };
    }

    /// The tags listings carry, the most used first, with how many carry each.
    pub fn tags(&self) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for t in self.listings.iter().flat_map(|l| &l.app_tags) {
            *counts.entry(t.as_str()).or_default() += 1;
        }
        let mut tags: Vec<(String, usize)> = counts
            .into_iter()
            .map(|(t, n)| (t.to_string(), n))
            .collect();
        tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(name: &str, tags: &[&str]) -> Listing {
        serde_json::from_value(serde_json::json!({
            "uri": format!("at://did:plc:a/fyi.atstore.listing.detail/{name}"),
            "name": name, "tagline": "A place", "description": "Long words",
            "appTags": tags, "rating": "4.8", "reviewCount": 3,
            "productAccountHandle": "app.example", "categorySlug": "ignored"
        }))
        .unwrap()
    }

    #[test]
    fn reads_atstore_listings_and_their_rating() {
        let l = listing("Mu", &["social"]);
        assert_eq!(l.stars(), Some(4.8));
        assert_eq!(l.icon_url, None);
        let unrated = Listing {
            review_count: 0,
            ..l.clone()
        };
        assert_eq!(unrated.stars(), None, "no reviews, no rating");
        let back: Listing = serde_json::from_str(&serde_json::to_string(&l).unwrap()).unwrap();
        assert_eq!(back, l);
    }

    #[test]
    fn search_needs_every_word_anywhere() {
        let l = listing("Witchsky", &["bluesky client"]);
        assert!(l.matches(""));
        assert!(l.matches("WITCH"));
        assert!(l.matches("client place"));
        assert!(l.matches("app.example"));
        assert!(!l.matches("client chess"));
    }

    #[test]
    fn tags_are_counted_most_used_first() {
        let c = Catalog {
            listings: vec![
                listing("A", &["social", "games"]),
                listing("B", &["social"]),
                listing("C", &["art"]),
            ],
            ..Catalog::default()
        };
        let tags = c.tags();
        assert_eq!(tags[0], ("social".to_string(), 2));
        assert_eq!(tags[1].0, "art", "ties go by name");
    }

    #[test]
    fn a_newer_read_keeps_pages_of_listings_still_listed() {
        let (a, b) = (listing("A", &[]), listing("B", &[]));
        let page = |l: &Listing| ListingDetail {
            listing: l.clone(),
            screenshots: Vec::new(),
            external_url: None,
            links: Vec::new(),
        };
        let mut c = Catalog {
            fetched: "2026-10-01T00:00:00Z".into(),
            listings: vec![a.clone(), b.clone()],
            details: [(a.uri.clone(), page(&a)), (b.uri.clone(), page(&b))].into(),
        };
        c.replace(Catalog {
            fetched: "2026-10-02T00:00:00Z".into(),
            listings: vec![a.clone()],
            details: BTreeMap::new(),
        });
        assert_eq!(c.fetched, "2026-10-02T00:00:00Z");
        assert_eq!(c.details.len(), 1);
        assert!(c.details.contains_key(&a.uri));
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Catalog>(&json).unwrap(), c);
    }

    #[test]
    fn age_is_counted_from_the_last_read() {
        let c = Catalog {
            fetched: "2026-10-14T00:00:00Z".into(),
            ..Catalog::default()
        };
        assert!(!c.older_than(12, "2026-10-14T11:00:00Z"));
        assert!(c.older_than(12, "2026-10-14T13:00:00Z"));
        assert!(Catalog::default().older_than(12, "2026-10-14T13:00:00Z"));
    }
}
