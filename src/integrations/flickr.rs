//! Flickr albums and group pools.

use super::{Integration, Shape, Shown, card, host, parts};
use url::Url;

pub struct Flickr;

impl Integration for Flickr {
    fn id(&self) -> &'static str {
        "flickr"
    }

    fn name(&self) -> &'static str {
        "Flickr"
    }

    fn description(&self) -> &'static str {
        "Show Flickr albums and groups as cards."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        let what = match (host(url)?.as_str(), parts(url).as_slice()) {
            ("flickr.com", ["photos", _, "albums", _]) | ("flic.kr", ["s", _]) => "Album",
            ("flickr.com", ["groups", _, "pool"]) | ("flic.kr", ["go", _]) => "Group",
            _ => return None,
        };
        Some(card("Flickr", what, "View", Shape::Banner).open(url, title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn what(u: &str) -> Option<String> {
        match Flickr.embed(&Url::parse(u).unwrap(), None)? {
            Shown::Card(c) => c.subtitle,
            Shown::Gif(_) => None,
        }
    }

    #[test]
    fn recognises_albums_and_groups() {
        assert_eq!(
            what("https://www.flickr.com/photos/me/albums/72157").as_deref(),
            Some("Album")
        );
        assert_eq!(what("https://flic.kr/s/aHsk").as_deref(), Some("Album"));
        assert_eq!(
            what("https://flickr.com/groups/abc/pool").as_deref(),
            Some("Group")
        );
        assert_eq!(what("https://flic.kr/go/8WJt").as_deref(), Some("Group"));
        assert_eq!(what("https://flickr.com/photos/me/123"), None);
    }
}
