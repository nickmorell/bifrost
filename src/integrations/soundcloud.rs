//! SoundCloud tracks and sets.

use super::{Integration, Shape, Shown, card, host, parts};
use url::Url;

pub struct SoundCloud;

impl Integration for SoundCloud {
    fn id(&self) -> &'static str {
        "soundcloud"
    }

    fn name(&self) -> &'static str {
        "SoundCloud"
    }

    fn description(&self) -> &'static str {
        "Show SoundCloud tracks and sets as cards."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        if host(url)? != "soundcloud.com" {
            return None;
        }
        let what = match parts(url).as_slice() {
            [_, "sets", _] => "Playlist",
            [_, track] if *track != "sets" => "Track",
            _ => return None,
        };
        Some(card("SoundCloud", what, "Listen", Shape::Cover).open(url, title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn what(u: &str) -> Option<String> {
        match SoundCloud.embed(&Url::parse(u).unwrap(), None)? {
            Shown::Card(c) => c.subtitle,
            Shown::Gif(_) => None,
        }
    }

    #[test]
    fn recognises_tracks_and_sets() {
        assert_eq!(
            what("https://soundcloud.com/artist/song").as_deref(),
            Some("Track")
        );
        assert_eq!(
            what("https://soundcloud.com/artist/sets/mix").as_deref(),
            Some("Playlist")
        );
        assert_eq!(what("https://soundcloud.com/artist"), None);
        assert_eq!(what("https://soundcloud.com/artist/sets"), None);
    }
}
