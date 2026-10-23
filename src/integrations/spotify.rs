//! Spotify songs, albums, playlists and podcasts, with their name and cover art read
//! from Spotify's oEmbed service, which needs no sign-in.

use super::{Details, Integration, Lookup, Shape, Shown, card, host, parts};
use std::time::Duration;
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
        "Show Spotify songs, albums, playlists and podcasts as cards, with cover art from Spotify."
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

    fn lookup(&self, url: &Url) -> Option<Lookup> {
        let mut oembed = Url::parse("https://open.spotify.com/oembed").ok()?;
        oembed.query_pairs_mut().append_pair("url", url.as_str());
        Some(Lookup {
            url: oembed.into(),
            keep: Duration::from_secs(24 * 60 * 60),
        })
    }

    fn details(&self, _url: &Url, body: &[u8]) -> Option<Details> {
        #[derive(serde::Deserialize)]
        struct OEmbed {
            title: Option<String>,
            thumbnail_url: Option<String>,
        }
        let o: OEmbed = serde_json::from_slice(body).ok()?;
        Some(Details {
            title: o.title.filter(|t| !t.trim().is_empty()),
            thumbnail_url: o.thumbnail_url,
            ..Details::default()
        })
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

    #[test]
    fn reads_the_name_and_cover_from_oembed() {
        let link = Url::parse("https://open.spotify.com/track/1?si=a&b=c").unwrap();
        assert_eq!(
            Spotify.lookup(&link).unwrap().url,
            "https://open.spotify.com/oembed?url=https%3A%2F%2Fopen.spotify.com%2Ftrack%2F1%3Fsi%3Da%26b%3Dc"
        );
        let body = br#"{"type":"rich","title":"A song","thumbnail_url":"https://i.scdn.co/image/x","html":"<iframe>"}"#;
        let d = Spotify.details(&link, body).unwrap();
        assert_eq!(d.title.as_deref(), Some("A song"));
        assert_eq!(
            d.thumbnail_url.as_deref(),
            Some("https://i.scdn.co/image/x")
        );
        assert_eq!(Spotify.details(&link, b"not json"), None);
    }
}
