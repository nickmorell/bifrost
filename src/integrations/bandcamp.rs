//! Bandcamp albums and tracks, on each artist's own `<name>.bandcamp.com`, and
//! reading the tracks off a release's page so they can be played here.

use super::{Integration, Shape, Shown, card, host, parts};
use serde_json::Value;
use url::Url;

pub struct Bandcamp;

impl Integration for Bandcamp {
    fn id(&self) -> &'static str {
        "bandcamp"
    }

    fn name(&self) -> &'static str {
        "Bandcamp"
    }

    fn description(&self) -> &'static str {
        "Show Bandcamp albums and tracks as cards you can play here."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        let host = host(url)?;
        if !host.strip_suffix(".bandcamp.com").is_some_and(is_artist) {
            return None;
        }
        let what = match parts(url).as_slice() {
            ["album", _, ..] => "Album",
            ["track", _, ..] => "Track",
            _ => return None,
        };
        Some(card("Bandcamp", what, "Listen", Shape::Cover).open(url, title))
    }
}

/// An album or track as its page describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub title: String,
    pub artist: String,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub title: String,
    /// Length in seconds.
    pub seconds: f64,
    /// Where the 128 kbit/s MP3 Bandcamp streams can be fetched, when the artist
    /// lets it be played.
    pub stream: Option<String>,
}

/// Reads the tracks from a release page's `data-tralbum` attribute. Fields that are
/// missing or null are left empty rather than failing the whole release.
pub fn release(html: &str) -> Option<Release> {
    let start = html.find("data-tralbum=\"")? + "data-tralbum=\"".len();
    let len = html[start..].find('"')?;
    let t: Value = serde_json::from_str(&unescape(&html[start..start + len])).ok()?;
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    Some(Release {
        title: text(&t["current"]["title"]),
        artist: text(&t["artist"]),
        tracks: t["trackinfo"]
            .as_array()?
            .iter()
            .map(|i| Track {
                title: text(&i["title"]),
                seconds: i["duration"].as_f64().unwrap_or_default(),
                stream: i["file"]["mp3-128"]
                    .as_str()
                    .filter(|u| is_stream(u))
                    .map(str::to_string),
            })
            .collect(),
    })
}

/// Bandcamp's own streaming servers, over https.
fn is_stream(url: &str) -> bool {
    Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str()
                .is_some_and(|h| h == "bcbits.com" || h.ends_with(".bcbits.com"))
    })
}

/// Turns HTML character references back into the characters they stand for.
fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Whether `url` is a Bandcamp album or track page.
pub fn is_release(url: &str) -> bool {
    Url::parse(url).is_ok_and(|u| Bandcamp.embed(&u, None).is_some())
}

/// An artist's subdomain: at least four letters, digits or inner hyphens.
fn is_artist(name: &str) -> bool {
    name.len() >= 4
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn what(u: &str) -> Option<String> {
        match Bandcamp.embed(&Url::parse(u).unwrap(), None)? {
            Shown::Card(c) => c.subtitle,
            Shown::Gif(_) => None,
        }
    }

    #[test]
    fn recognises_albums_and_tracks() {
        assert_eq!(
            what("https://some-band.bandcamp.com/album/x").as_deref(),
            Some("Album")
        );
        assert_eq!(
            what("https://someband.bandcamp.com/track/y").as_deref(),
            Some("Track")
        );
        assert_eq!(what("https://someband.bandcamp.com/"), None);
        assert_eq!(what("https://bandcamp.com/album/x"), None);
        assert_eq!(what("https://abc.bandcamp.com/album/x"), None);
        assert_eq!(what("https://a.b.bandcamp.com/album/x"), None);
        assert!(is_release("https://some-band.bandcamp.com/album/x"));
    }

    #[test]
    fn reads_the_tracks_off_a_release_page() {
        let html = r#"<div><script data-tralbum="{&quot;artist&quot;:&quot;Band &amp; Co&quot;,&quot;current&quot;:{&quot;title&quot;:&quot;Songs&quot;},&quot;trackinfo&quot;:[{&quot;title&quot;:&quot;One&quot;,&quot;duration&quot;:61.5,&quot;file&quot;:{&quot;mp3-128&quot;:&quot;https://t4.bcbits.com/stream/abc&quot;}},{&quot;title&quot;:&quot;Two&quot;,&quot;duration&quot;:30,&quot;file&quot;:null},{&quot;title&quot;:&quot;Three&quot;,&quot;duration&quot;:5,&quot;file&quot;:{&quot;mp3-128&quot;:&quot;http://evil.example/x&quot;}}]}" data-other="x"></script>"#;
        let r = release(html).unwrap();
        assert_eq!(r.artist, "Band & Co");
        assert_eq!(r.title, "Songs");
        assert_eq!(r.tracks.len(), 3);
        assert_eq!(
            r.tracks[0].stream.as_deref(),
            Some("https://t4.bcbits.com/stream/abc")
        );
        assert_eq!(r.tracks[1].stream, None);
        assert_eq!(r.tracks[2].stream, None, "only Bandcamp's own servers");
        assert!((r.tracks[0].seconds - 61.5).abs() < f64::EPSILON);
        assert_eq!(release("<html></html>"), None);
        let nulls = r#"<script data-tralbum="{&quot;artist&quot;:null,&quot;current&quot;:{&quot;title&quot;:null},&quot;trackinfo&quot;:[{&quot;title&quot;:null,&quot;duration&quot;:null,&quot;file&quot;:null}]}">"#;
        assert_eq!(release(nulls).unwrap().tracks.len(), 1);
    }
}
