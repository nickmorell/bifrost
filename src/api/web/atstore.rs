//! ATstore (atstore.fyi), the directory of apps and services on the AT Protocol,
//! through its public XRPC calls: every listing, page by page, and one listing's
//! page. Reading needs no account.

use crate::api::ApiError;
use crate::api::client::{ApiClient, Problem, status_text};
use crate::model::{AtUri, Listing, ListingDetail, Page};
use crate::net::Transport;
use serde::Deserialize;

/// Where ATstore answers.
pub const HOST: &str = "https://atstore.fyi";

/// Listings asked for per call: the most ATstore gives.
const PER_PAGE: &str = "100";
/// Pages read at most for the whole directory, so a server that keeps handing back
/// a cursor can't keep BiFrost reading forever.
const MAX_PAGES: usize = 30;

/// ATstore's public calls.
pub struct Directory<'a>(pub &'a dyn Transport);

impl ApiClient for Directory<'_> {
    fn base_url(&self) -> &'static str {
        HOST
    }

    fn transport(&self) -> &dyn Transport {
        self.0
    }

    fn error(&self, problem: Problem) -> ApiError {
        ApiError::Other(match problem {
            Problem::Unreachable(e) => format!("Couldn't reach ATstore: {e}"),
            Problem::TooBig => "ATstore sent more than expected".into(),
            Problem::Status(r) => format!("ATstore answered: {}", status_text(&r)),
        })
    }
}

impl Directory<'_> {
    /// A page of listings, the most popular first.
    pub async fn page(&self, cursor: Option<&str>) -> Result<Page<Listing>, ApiError> {
        #[derive(Deserialize)]
        struct Out {
            #[serde(default)]
            listings: Vec<serde_json::Value>,
            cursor: Option<String>,
        }
        let mut query = vec![("sort", "popular"), ("limit", PER_PAGE)];
        query.extend(cursor.map(|c| ("cursor", c)));
        let out: Out = self
            .get("/xrpc/fyi.atstore.directory.searchListings", &query)
            .await?;
        Ok(Page {
            // One listing ATstore words differently doesn't hide the rest.
            items: out
                .listings
                .into_iter()
                .filter_map(|l| serde_json::from_value(l).ok())
                .collect(),
            cursor: out.cursor.filter(|c| !c.is_empty()),
        })
    }

    /// Every listing, the most popular first.
    pub async fn all(&self) -> Result<Vec<Listing>, ApiError> {
        let mut listings: Vec<Listing> = Vec::new();
        let mut cursor = None;
        for _ in 0..MAX_PAGES {
            let page = self.page(cursor.as_deref()).await?;
            let added = page.items.len();
            for l in page.items {
                if !listings.iter().any(|seen| seen.uri == l.uri) {
                    listings.push(l);
                }
            }
            match page.cursor {
                Some(c) if added > 0 => cursor = Some(c),
                _ => break,
            }
        }
        Ok(listings)
    }

    /// One listing's page.
    pub async fn listing(&self, uri: &AtUri) -> Result<ListingDetail, ApiError> {
        self.get(
            "/xrpc/fyi.atstore.directory.getListing",
            &[("uri", uri.as_str())],
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fake::Fake;

    const PAGE: &str = r#"{"listings":[
        {"name":"Witchsky","tagline":"Bluesky app","description":"Themes",
         "iconUrl":"https://cdn.bsky.app/img/x@png","categorySlug":"apps/witchsky",
         "category":"Witchsky","accent":"blue","rating":"5","reviewCount":26,
         "priceLabel":"GET","productAccountHandle":"witchsky.app",
         "appTags":["bluesky client","social"],
         "uri":"at://did:plc:w/fyi.atstore.listing.detail/1"},
        {"broken":true}],"cursor":"eyJvIjoyNH0"}"#;

    #[tokio::test]
    async fn reads_a_page_of_listings() {
        let net = Fake::default().reply(200, PAGE);
        let page = Directory(&net).page(None).await.unwrap();
        assert_eq!(page.items.len(), 1, "the broken one is left out");
        let l = &page.items[0];
        assert_eq!(l.name, "Witchsky");
        assert_eq!(l.app_tags, ["bluesky client", "social"]);
        assert_eq!(l.stars(), Some(5.0));
        assert_eq!(page.cursor.as_deref(), Some("eyJvIjoyNH0"));
        assert_eq!(
            net.seen()[0].url.as_str(),
            "https://atstore.fyi/xrpc/fyi.atstore.directory.searchListings?sort=popular&limit=100"
        );
    }

    #[tokio::test]
    async fn reads_every_page_until_there_are_no_more() {
        let last = PAGE
            .replace("/1\"", "/2\"")
            .replace(",\"cursor\":\"eyJvIjoyNH0\"", "");
        let net = Fake::default().reply(200, PAGE).reply(200, &last);
        let all = Directory(&net).all().await.unwrap();
        assert_eq!(all.len(), 2);
        let seen = net.seen();
        assert_eq!(seen.len(), 2);
        assert!(seen[1].url.as_str().ends_with("&cursor=eyJvIjoyNH0"));
    }

    #[tokio::test]
    async fn stops_when_a_page_brings_nothing_new() {
        let empty = r#"{"listings":[],"cursor":"more"}"#;
        let net = Fake::default().reply(200, PAGE).reply(200, empty);
        assert_eq!(Directory(&net).all().await.unwrap().len(), 1);
        assert_eq!(net.seen().len(), 2);
    }

    #[tokio::test]
    async fn reads_a_listing_page() {
        let one = r#"{"listing":{"name":"Mu","uri":"at://did:plc:m/fyi.atstore.listing.detail/3"},
            "isStoreManaged":false,"screenshots":["https://cdn.bsky.app/img/s@jpeg"],
            "externalUrl":"https://mu.social/","links":[{"uri":"https://mu.social/privacy"}]}"#;
        let net = Fake::default().reply(200, one);
        let uri = AtUri::from("at://did:plc:m/fyi.atstore.listing.detail/3");
        let d = Directory(&net).listing(&uri).await.unwrap();
        assert_eq!(d.listing.name, "Mu");
        assert_eq!(d.external_url.as_deref(), Some("https://mu.social/"));
        assert_eq!(d.links[0].uri, "https://mu.social/privacy");
        assert_eq!(d.screenshots.len(), 1);
        assert_eq!(
            net.seen()[0].url.query(),
            Some("uri=at%3A%2F%2Fdid%3Aplc%3Am%2Ffyi.atstore.listing.detail%2F3")
        );
    }

    #[tokio::test]
    async fn words_failures_as_atstore() {
        let net = Fake::default().reply(400, r#"{"error":"InvalidParams"}"#);
        let e = Directory(&net).page(None).await.unwrap_err().to_string();
        assert!(e.starts_with("ATstore answered"), "{e}");
    }
}
