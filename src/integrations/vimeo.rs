//! Vimeo videos.

use super::{Integration, Shape, Shown, card, host, parts};
use url::Url;

pub struct Vimeo;

impl Integration for Vimeo {
    fn id(&self) -> &'static str {
        "vimeo"
    }

    fn name(&self) -> &'static str {
        "Vimeo"
    }

    fn description(&self) -> &'static str {
        "Show Vimeo links as video cards."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        match (host(url)?.as_str(), parts(url).as_slice()) {
            ("vimeo.com", [id]) if id.bytes().all(|b| b.is_ascii_digit()) => {
                Some(card("Vimeo", "Video", "Watch", Shape::Banner).open(url, title))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_videos_only() {
        let e = |u: &str| Vimeo.embed(&Url::parse(u).unwrap(), None).is_some();
        assert!(e("https://vimeo.com/76979871"));
        assert!(e("https://www.vimeo.com/76979871"));
        assert!(!e("https://vimeo.com/channels/staffpicks"));
        assert!(!e("https://vimeo.com/about"));
    }
}
