//! Streamplace, live streaming on the AT Protocol. Turning it on adds the Live screen
//! (who is streaming, streamer pages with their stream and chat), opens stream.place
//! links in BiFrost, and shows them as cards in posts.

use super::{Group, Integration, Shape, Shown, card, host, parts};
use url::Url;

pub struct Streamplace;

/// The id Streamplace's switch is kept under.
pub const ID: &str = "streamplace";

impl Integration for Streamplace {
    fn id(&self) -> &'static str {
        ID
    }

    fn name(&self) -> &'static str {
        "Streamplace"
    }

    fn description(&self) -> &'static str {
        "Watch live streams and chat on stream.place, from a Live screen in the sidebar. Opens stream.place links here."
    }

    fn group(&self) -> Group {
        Group::Service
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        match (host(url)?.as_str(), parts(url).as_slice()) {
            ("stream.place", [who]) if who.contains('.') || who.starts_with("did:") => {
                Some(card("Streamplace", "Stream", "Watch", Shape::Banner).open(url, title))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(url: &str) -> Option<Shown> {
        Streamplace.embed(&Url::parse(url).unwrap(), None)
    }

    #[test]
    fn streamer_links_become_cards() {
        assert!(shown("https://stream.place/iame.li").is_some());
        assert!(shown("https://stream.place/did:plc:abc").is_some());
        assert!(shown("https://stream.place/about").is_none());
        assert!(shown("https://stream.place/iame.li/extra").is_none());
        assert!(shown("https://example.com/iame.li").is_none());
    }
}
