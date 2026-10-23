//! Twitch channels, past broadcasts and clips, and whether a channel is live. That is
//! read from the channel's own page, which describes a stream that is on for search
//! engines; Twitch's API would need a key of BiFrost's own.

use super::{Details, Integration, Lookup, Shape, Shown, card, host, parts};
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub struct Twitch;

impl Integration for Twitch {
    fn id(&self) -> &'static str {
        "twitch"
    }

    fn name(&self) -> &'static str {
        "Twitch"
    }

    fn description(&self) -> &'static str {
        "Show Twitch channels, videos and clips as cards, marking channels that are live now."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        let what = match (host(url)?.as_str(), parts(url).as_slice()) {
            ("twitch.tv", ["videos", _]) => "Video",
            ("twitch.tv", [_, "clip", _]) | ("clips.twitch.tv", [_]) => "Clip",
            _ if channel(url).is_some() => "Channel",
            _ => return None,
        };
        Some(card("Twitch", what, "Watch", Shape::Banner).open(url, title))
    }

    fn lookup(&self, url: &Url) -> Option<Lookup> {
        let channel = channel(url)?;
        Some(Lookup {
            url: format!("https://www.twitch.tv/{channel}"),
            keep: Duration::from_secs(5 * 60),
        })
    }

    /// A channel's page says it is live with a `BroadcastEvent` in its structured data,
    /// along with the stream's title and preview pictures. A page without one is a
    /// channel that is offline.
    fn details(&self, _url: &Url, body: &[u8]) -> Option<Details> {
        let html = std::str::from_utf8(body).ok()?;
        Some(live(html).unwrap_or_default())
    }
}

/// The channel a link is to, when it is to a channel's page.
fn channel(url: &Url) -> Option<&str> {
    match (host(url)?.as_str(), parts(url).as_slice()) {
        ("twitch.tv", [channel]) if is_channel(channel) => Some(channel),
        _ => None,
    }
}

fn live(html: &str) -> Option<Details> {
    const OPEN: &str = r#"<script type="application/ld+json">"#;
    let mut rest = html;
    while let Some(start) = rest.find(OPEN) {
        let after = &rest[start + OPEN.len()..];
        let end = after.find("</script>")?;
        if let Ok(data) = serde_json::from_str::<Value>(&after[..end]) {
            let items = data.as_array().cloned().unwrap_or_else(|| vec![data]);
            if let Some(item) = items.iter().find(|i| is_live(&i["publication"])) {
                let pictures = &item["thumbnailUrl"];
                let picture = pictures
                    .as_array()
                    .and_then(|p| p.last())
                    .unwrap_or(pictures)
                    .as_str();
                return Some(Details {
                    text: item["description"].as_str().map(str::to_string),
                    thumbnail_url: picture.map(str::to_string),
                    live: true,
                    ..Details::default()
                });
            }
        }
        rest = &after[end..];
    }
    None
}

/// Whether a `publication`, one event or a list of them, is a broadcast that is on.
fn is_live(publication: &Value) -> bool {
    match publication {
        Value::Array(events) => events.iter().any(is_live),
        event => event["isLiveBroadcast"].as_bool() == Some(true) && event["endDate"].is_null(),
    }
}

/// Twitch channel names: letters, digits and underscores.
fn is_channel(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn what(u: &str) -> Option<String> {
        match Twitch.embed(&Url::parse(u).unwrap(), None)? {
            Shown::Card(c) => c.subtitle,
            Shown::Gif(_) => None,
        }
    }

    #[test]
    fn recognises_channels_videos_and_clips() {
        assert_eq!(
            what("https://www.twitch.tv/some_one").as_deref(),
            Some("Channel")
        );
        assert_eq!(
            what("https://twitch.tv/videos/123").as_deref(),
            Some("Video")
        );
        assert_eq!(
            what("https://m.twitch.tv/a/clip/Funny-Clip").as_deref(),
            Some("Clip")
        );
        assert_eq!(
            what("https://clips.twitch.tv/Funny-Clip").as_deref(),
            Some("Clip")
        );
        assert_eq!(what("https://twitch.tv/"), None);
        assert_eq!(what("https://twitch.tv/a/about/more"), None);
    }

    #[test]
    fn only_channels_are_checked_for_being_live() {
        let l = |u: &str| Twitch.lookup(&Url::parse(u).unwrap()).map(|l| l.url);
        assert_eq!(
            l("https://m.twitch.tv/some_one").as_deref(),
            Some("https://www.twitch.tv/some_one")
        );
        assert_eq!(l("https://twitch.tv/videos/123"), None);
        assert_eq!(l("https://clips.twitch.tv/Funny-Clip"), None);
    }

    #[test]
    fn reads_a_live_stream_off_the_channel_page() {
        let link = Url::parse("https://www.twitch.tv/some_one").unwrap();
        let page = r#"<html><script type="application/ld+json">{"@type":"WebSite"}</script>
            <script type="application/ld+json">[{"@type":"VideoObject","description":"Playing a game",
            "thumbnailUrl":["https://static-cdn.jtvnw.net/a-320x180.jpg","https://static-cdn.jtvnw.net/a-1280x720.jpg"],
            "publication":{"@type":"BroadcastEvent","isLiveBroadcast":true,"startDate":"2026-10-23T20:00:00Z"}}]</script></html>"#;
        let d = Twitch.details(&link, page.as_bytes()).unwrap();
        assert!(d.live);
        assert_eq!(d.text.as_deref(), Some("Playing a game"));
        assert_eq!(
            d.thumbnail_url.as_deref(),
            Some("https://static-cdn.jtvnw.net/a-1280x720.jpg")
        );
        let ended = page.replace(
            r#""startDate""#,
            r#""endDate":"2026-10-23T21:00:00Z","startDate""#,
        );
        assert_eq!(
            Twitch.details(&link, ended.as_bytes()),
            Some(Details::default())
        );
        let offline = "<html><title>some_one - Twitch</title></html>";
        assert_eq!(
            Twitch.details(&link, offline.as_bytes()),
            Some(Details::default())
        );
    }
}
