//! Live streams: who is streaming on Streamplace, their chat and past streams, and the
//! "live now" status Bluesky shows on the avatars of people streaming anywhere.

use super::{AtUri, Author, Did, PostRecord};
use serde::Deserialize;

/// Services whose links count for a live status, as in Bluesky, with their names. A
/// link counts on the service's own domain or any of its subdomains.
const LIVE_SERVICES: [(&str, &str); 6] = [
    ("stream.place", "Streamplace"),
    ("twitch.tv", "Twitch"),
    ("youtube.com", "YouTube"),
    ("bluecast.app", "Bluecast"),
    ("substack.com", "Substack"),
    ("beehiiv.com", "Beehiiv"),
];

/// A status on a Bluesky profile (`app.bsky.actor.defs#statusView`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorStatus {
    #[serde(default)]
    pub status: String,
    pub embed: Option<StatusEmbed>,
    /// When the status runs out, as an RFC 3339 time.
    pub expires_at: Option<String>,
    pub is_active: Option<bool>,
    #[serde(default)]
    pub is_disabled: bool,
}

/// The link a status points at (`app.bsky.embed.external#view`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StatusEmbed {
    pub external: Option<StatusLink>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StatusLink {
    pub uri: String,
    #[serde(default)]
    pub title: String,
}

/// Someone streaming now: where, and what.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveNow<'a> {
    pub url: &'a str,
    pub title: &'a str,
    /// The service's name, such as "Twitch".
    pub service: &'static str,
}

impl ActorStatus {
    /// Where this person is streaming, if the status says they are live now (`now` as
    /// an RFC 3339 time), on a service Bluesky accepts live links from.
    pub fn live_now(&self, now: &str) -> Option<LiveNow<'_>> {
        if self.status != "app.bsky.actor.status#live"
            || self.is_disabled
            || self.is_active == Some(false)
        {
            return None;
        }
        let expires = chrono::DateTime::parse_from_rfc3339(self.expires_at.as_deref()?).ok()?;
        let now = chrono::DateTime::parse_from_rfc3339(now).ok()?;
        if expires <= now {
            return None;
        }
        let link = self.embed.as_ref()?.external.as_ref()?;
        let host = url::Url::parse(&link.uri)
            .ok()?
            .host_str()?
            .to_ascii_lowercase();
        let service = LIVE_SERVICES.iter().find_map(|(domain, name)| {
            let sub = host.strip_suffix(domain)?;
            (sub.is_empty() || sub.ends_with('.')).then_some(*name)
        })?;
        Some(LiveNow {
            url: &link.uri,
            title: &link.title,
            service,
        })
    }
}

/// A Streamplace stream: one `place.stream.livestream` record and who made it.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveStream {
    pub uri: AtUri,
    pub did: Did,
    pub handle: String,
    pub title: String,
    /// When it started, as an RFC 3339 time.
    pub started: String,
    /// When it ended; a stream that has ended doesn't start again.
    pub ended: Option<String>,
    /// People watching now, when the node said.
    pub viewers: Option<u64>,
    /// A still from the stream.
    pub thumb: Option<String>,
    /// What is on, such as a game's name or "Software dev".
    pub activity: Option<String>,
    pub tags: Vec<String>,
}

/// A message in a stream's chat (`place.stream.chat.defs#messageView`).
#[derive(Debug, Clone)]
pub struct LiveChat {
    pub uri: AtUri,
    pub author: Author,
    /// Its text and links; `created_at` is when it was sent.
    pub body: PostRecord,
    /// The colour the author picked for their name, as red, green and blue.
    pub color: Option<[u8; 3]>,
    /// Who and what this message answers: (name, text).
    pub reply_to: Option<(String, String)>,
}

/// Something that happened in a stream, as its node reports it.
#[derive(Debug, Clone)]
pub enum LiveEvent {
    /// A new message, or one sent again with changes.
    Chat(LiveChat),
    /// A message was deleted by its author or hidden by the streamer.
    Removed(AtUri),
    /// The streamer blocked someone; their messages go.
    Blocked(Did),
    Viewers(u64),
    /// The stream's record changed: a new title, or it ended.
    Stream(LiveStream),
    /// The pinned message, or none.
    Pinned(Option<LiveChat>),
    /// The connection to the stream's chat dropped or came back.
    Connected(bool),
}

/// A past stream, kept as a video (`place.stream.video`).
#[derive(Debug, Clone, PartialEq)]
pub struct PastStream {
    pub uri: AtUri,
    pub title: String,
    /// When it was made, as an RFC 3339 time.
    pub created: String,
    /// Its length in seconds, when known.
    pub seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(json: &str) -> ActorStatus {
        serde_json::from_str(json).unwrap()
    }

    const LIVE: &str = r#"{"status":"app.bsky.actor.status#live",
        "embed":{"$type":"app.bsky.embed.external#view","external":{
            "uri":"https://stream.place/iame.li","title":"Reading the list"}},
        "expiresAt":"2026-10-07T21:47:30.000Z","isActive":true}"#;

    #[test]
    fn a_current_live_status_says_where() {
        let s = status(LIVE);
        let live = s.live_now("2026-10-07T21:40:00Z").unwrap();
        assert_eq!(live.url, "https://stream.place/iame.li");
        assert_eq!(live.title, "Reading the list");
        assert_eq!(live.service, "Streamplace");
    }

    #[test]
    fn expired_inactive_or_unknown_statuses_dont_count() {
        let s = status(LIVE);
        assert!(s.live_now("2026-10-07T21:50:00Z").is_none(), "expired");
        let off = status(&LIVE.replace("\"isActive\":true", "\"isActive\":false"));
        assert!(off.live_now("2026-10-07T21:40:00Z").is_none());
        let elsewhere = status(&LIVE.replace("stream.place", "evil.example"));
        assert!(elsewhere.live_now("2026-10-07T21:40:00Z").is_none());
        let lookalike = status(&LIVE.replace("stream.place", "notstream.place"));
        assert!(lookalike.live_now("2026-10-07T21:40:00Z").is_none());
        let sub = status(&LIVE.replace("https://stream.place", "https://m.twitch.tv"));
        assert_eq!(
            sub.live_now("2026-10-07T21:40:00Z").unwrap().service,
            "Twitch"
        );
        assert!(
            ActorStatus::default()
                .live_now("2026-10-07T21:40:00Z")
                .is_none()
        );
    }
}
