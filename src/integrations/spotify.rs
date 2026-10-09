//! Spotify songs, albums, playlists and podcasts.

use super::{Integration, Shape, Shown, card, host, parts};
use url::Url;

pub struct Spotify;

impl Integration for Spotify {
    fn id(&self) -> &'static str {
        "spotify"
    }

    fn name(&self) -> &'static str {
        "Spotify"
    }

    fn description(&self) -> &'static str {
        "Show Spotify songs, albums, playlists and podcasts as cards."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        if host(url)? != "open.spotify.com" {
            return None;
        }
        let segs = parts(url);
        // Links shared from some countries start with a locale, such as `/intl-de/`.
        let segs = match segs.as_slice() {
            [first, rest @ ..] if first.starts_with("intl-") => rest,
            all => all,
        };
        let what = match segs {
            ["track", _] => "Song",
            ["album", _] => "Album",
            ["playlist", _] => "Playlist",
            ["episode", _] => "Episode",
            ["show", _] => "Podcast",
            _ => return None,
        };
        Some(card("Spotify", what, "Listen", Shape::Cover).open(url, title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn what(u: &str) -> Option<String> {
        match Spotify.embed(&Url::parse(u).unwrap(), None)? {
            Shown::Card(c) => c.subtitle,
            Shown::Gif(_) => None,
        }
    }

    #[test]
    fn recognises_each_kind() {
        let base = "https://open.spotify.com";
        assert_eq!(what(&format!("{base}/track/1")).as_deref(), Some("Song"));
        assert_eq!(
            what(&format!("{base}/intl-de/album/1?si=x")).as_deref(),
            Some("Album")
        );
        assert_eq!(
            what(&format!("{base}/playlist/1")).as_deref(),
            Some("Playlist")
        );
        assert_eq!(
            what(&format!("{base}/episode/1")).as_deref(),
            Some("Episode")
        );
        assert_eq!(what(&format!("{base}/show/1")).as_deref(), Some("Podcast"));
        assert_eq!(what(&format!("{base}/artist/1")), None);
        assert_eq!(what("https://spotify.com/track/1"), None);
    }
}
