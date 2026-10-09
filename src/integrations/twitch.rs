//! Twitch channels, past broadcasts and clips.

use super::{Integration, Shape, Shown, card, host, parts};
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
        "Show Twitch channels, videos and clips as cards."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        let what = match (host(url)?.as_str(), parts(url).as_slice()) {
            ("twitch.tv", ["videos", _]) => "Video",
            ("twitch.tv", [_, "clip", _]) | ("clips.twitch.tv", [_]) => "Clip",
            ("twitch.tv", [channel]) if is_channel(channel) => "Channel",
            _ => return None,
        };
        Some(card("Twitch", what, "Watch", Shape::Banner).open(url, title))
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
}
