//! YouTube videos and YouTube Shorts, two sources as in Bluesky: a thumbnail and a
//! watch button instead of a plain link card. Needs no account; thumbnails come from
//! YouTube's public image host.

use super::{Integration, Shape, Shown, card, host, parts};
use url::Url;

pub struct YouTube;

pub struct Shorts;

impl Integration for YouTube {
    fn id(&self) -> &'static str {
        "youtube"
    }

    fn name(&self) -> &'static str {
        "YouTube"
    }

    fn description(&self) -> &'static str {
        "Show YouTube links as video cards with a thumbnail. Loads thumbnails from i.ytimg.com."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        match video(url)? {
            (id, false) => shown(&id, "Video", title),
            (_, true) => None,
        }
    }
}

impl Integration for Shorts {
    fn id(&self) -> &'static str {
        "youtube_shorts"
    }

    fn name(&self) -> &'static str {
        "YouTube Shorts"
    }

    fn description(&self) -> &'static str {
        "Show YouTube Shorts links as video cards with a thumbnail. Loads thumbnails from i.ytimg.com."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        match video(url)? {
            (id, true) => shown(&id, "Short", title),
            (_, false) => None,
        }
    }
}

fn shown(id: &str, what: &str, title: Option<&str>) -> Option<Shown> {
    let watch = Url::parse(&format!("https://www.youtube.com/watch?v={id}")).ok()?;
    Some(
        card("YouTube", what, "Watch", Shape::Banner)
            .thumbnail(format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"))
            .open(&watch, title),
    )
}

/// The video's id, and whether it is a Short.
fn video(url: &Url) -> Option<(String, bool)> {
    let host = host(url)?;
    let segs = parts(url);
    let (id, short) = match (host.as_str(), segs.as_slice()) {
        ("youtu.be", [id, ..]) => ((*id).to_string(), false),
        ("youtube.com" | "music.youtube.com", ["watch", ..]) => (
            url.query_pairs().find(|(k, _)| k == "v")?.1.into_owned(),
            false,
        ),
        ("youtube.com" | "music.youtube.com", ["shorts", id, ..]) => ((*id).to_string(), true),
        ("youtube.com" | "music.youtube.com", ["live" | "embed", id, ..]) => {
            ((*id).to_string(), false)
        }
        _ => return None,
    };
    (id.len() == 11
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
    .then_some((id, short))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(u: &str) -> Option<(String, bool)> {
        video(&Url::parse(u).unwrap())
    }

    #[test]
    fn recognises_youtube_urls() {
        let v = |s: &str| Some((s.to_string(), false));
        assert_eq!(id("https://youtu.be/dQw4w9WgXcQ?t=3"), v("dQw4w9WgXcQ"));
        assert_eq!(
            id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=x"),
            v("dQw4w9WgXcQ")
        );
        assert_eq!(id("https://youtube.com/live/dQw4w9WgXcQ"), v("dQw4w9WgXcQ"));
        assert_eq!(
            id("https://m.youtube.com/shorts/dQw4w9WgXcQ"),
            Some(("dQw4w9WgXcQ".into(), true))
        );
        assert_eq!(id("https://youtube.com/@channel"), None);
        assert_eq!(id("https://example.com/watch?v=dQw4w9WgXcQ"), None);
        assert_eq!(id("https://youtu.be/short"), None);
    }

    #[test]
    fn videos_and_shorts_are_separate_sources() {
        let video = Url::parse("https://youtu.be/dQw4w9WgXcQ").unwrap();
        let short = Url::parse("https://youtube.com/shorts/dQw4w9WgXcQ").unwrap();
        assert!(YouTube.embed(&video, None).is_some());
        assert!(YouTube.embed(&short, None).is_none());
        assert!(Shorts.embed(&short, None).is_some());
        assert!(Shorts.embed(&video, None).is_none());
    }

    #[test]
    fn cards_open_the_watch_page_with_a_thumbnail() {
        let url = Url::parse("https://youtu.be/dQw4w9WgXcQ?t=3").unwrap();
        let Some(Shown::Card(c)) = YouTube.embed(&url, Some("Song")) else {
            panic!("no card");
        };
        assert_eq!(c.open_url, "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert_eq!(
            c.thumbnail_url.as_deref(),
            Some("https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg")
        );
        assert_eq!(c.action_label, "Watch on YouTube");
    }
}
