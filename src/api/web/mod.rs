//! Public web resources: link previews through Bluesky's card service, plain
//! downloads such as images, the ATstore directory, and BiFrost's own GitHub
//! repository.

use super::client::ApiClient;
use super::{ApiError, LinkCard};
use crate::net::{Method, Transport, shared};

pub mod atstore;
pub mod github;

/// Bluesky's link preview service, as the official app uses.
pub struct CardService<'a>(pub &'a dyn Transport);

impl ApiClient for CardService<'_> {
    fn base_url(&self) -> &'static str {
        "https://cardyb.bsky.app"
    }

    fn transport(&self) -> &dyn Transport {
        self.0
    }
}

impl CardService<'_> {
    pub async fn card(&self, url: &str) -> Result<LinkCard, ApiError> {
        #[derive(serde::Deserialize)]
        struct Card {
            #[serde(default)]
            error: String,
            #[serde(default)]
            title: String,
            #[serde(default)]
            description: String,
            #[serde(default)]
            image: String,
        }
        let card: Card = self.get("/v1/extract", &[("url", url)]).await?;
        if !card.error.is_empty() {
            return Err(ApiError::Other(card.error));
        }
        Ok(LinkCard {
            url: url.to_string(),
            title: card.title,
            description: card.description,
            image_url: (!card.image.is_empty()).then_some(card.image),
        })
    }
}

/// Any public address, given whole.
pub struct Web<'a>(pub &'a dyn Transport);

impl ApiClient for Web<'_> {
    fn base_url(&self) -> &'static str {
        ""
    }

    fn transport(&self) -> &dyn Transport {
        self.0
    }
}

impl Web<'_> {
    pub async fn download(&self, url: &str) -> Result<Vec<u8>, ApiError> {
        self.fetch(self.request(Method::Get, url, &[])?).await
    }
}

/// Fetches link preview metadata.
pub async fn link_card(url: &str) -> Result<LinkCard, ApiError> {
    CardService(shared()).card(url).await
}

/// Downloads an image or other public resource, up to 25 MB.
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, ApiError> {
    Web(shared()).download(url).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fake::Fake;

    #[tokio::test]
    async fn reads_link_cards() {
        let net = Fake::default()
            .reply(
                200,
                r#"{"title":"T","description":"D","image":"https://i/x.jpg"}"#,
            )
            .reply(200, r#"{"error":"Unable to generate link preview"}"#);
        let card = CardService(&net).card("https://a.example/").await.unwrap();
        assert_eq!(card.title, "T");
        assert_eq!(card.image_url.as_deref(), Some("https://i/x.jpg"));
        assert_eq!(
            net.seen()[0].url.as_str(),
            "https://cardyb.bsky.app/v1/extract?url=https%3A%2F%2Fa.example%2F"
        );
        let err = CardService(&net)
            .card("https://b.example/")
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "Unable to generate link preview");
    }

    #[tokio::test]
    async fn downloads_whole_addresses() {
        let net = Fake::default().reply(200, "bytes").reply(404, "");
        let web = Web(&net);
        assert_eq!(
            web.download("https://a.example/x.png").await.unwrap(),
            b"bytes"
        );
        assert_eq!(net.seen()[0].url.as_str(), "https://a.example/x.png");
        assert_eq!(net.seen()[0].limit, 25 * 1024 * 1024);
        let err = web.download("https://a.example/y.png").await.unwrap_err();
        assert_eq!(err.to_string(), "the server answered 404 Not Found");
    }
}
