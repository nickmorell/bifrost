//! Apple Music songs, albums and playlists.

use super::{Integration, Shape, Shown, card, host, parts};
use url::Url;

pub struct AppleMusic;

impl Integration for AppleMusic {
    fn id(&self) -> &'static str {
        "apple_music"
    }

    fn name(&self) -> &'static str {
        "Apple Music"
    }

    fn description(&self) -> &'static str {
        "Show Apple Music songs, albums and playlists as cards."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        if host(url)? != "music.apple.com" {
            return None;
        }
        // `/<country>/<kind>/<name>/<id>`; an album link with `?i=` is one of its songs.
        let song_in_album = url.query_pairs().any(|(k, _)| k == "i");
        let what = match parts(url).as_slice() {
            [_, "album", _, _] if song_in_album => "Song",
            [_, "album", _, _] => "Album",
            [_, "playlist", _, _] => "Playlist",
            [_, "song", _, _] => "Song",
            _ => return None,
        };
        Some(card("Apple Music", what, "Listen", Shape::Cover).open(url, title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn what(u: &str) -> Option<String> {
        match AppleMusic.embed(&Url::parse(u).unwrap(), None)? {
            Shown::Card(c) => c.subtitle,
            Shown::Gif(_) => None,
        }
    }

    #[test]
    fn recognises_each_kind() {
        let base = "https://music.apple.com/us";
        assert_eq!(
            what(&format!("{base}/album/name/123")).as_deref(),
            Some("Album")
        );
        assert_eq!(
            what(&format!("{base}/album/name/123?i=456")).as_deref(),
            Some("Song")
        );
        assert_eq!(
            what(&format!("{base}/song/name/123")).as_deref(),
            Some("Song")
        );
        assert_eq!(
            what(&format!("{base}/playlist/name/pl.1")).as_deref(),
            Some("Playlist")
        );
        assert_eq!(what(&format!("{base}/artist/name/123")), None);
        assert_eq!(what(&format!("{base}/album/123")), None);
    }
}
