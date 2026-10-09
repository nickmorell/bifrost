//! The live streams and streamer pages loaded so far.

use crate::model::{Author, Did, LiveChat, LiveStream, PastStream, Profile};
use std::collections::{HashMap, HashSet, VecDeque};

/// Chat messages kept per stream; older ones scroll away.
pub(super) const MAX_CHAT: usize = 300;
/// The longest chat message, in characters, as Streamplace allows.
pub const MAX_MESSAGE: usize = 300;

/// Everything the Live screens show.
#[derive(Debug, Default)]
pub struct Live {
    pub(super) board: Board,
    /// Streamer pages, by the handle or DID they were opened with.
    pub(super) channels: HashMap<String, Channel>,
}

/// Who is live now.
#[derive(Debug, Default)]
pub(super) struct Board {
    pub(super) streams: Vec<LiveStream>,
    pub(super) authors: HashMap<Did, Author>,
    pub(super) loading: bool,
    pub(super) loaded: bool,
    pub(super) error: Option<String>,
}

/// One streamer's page.
#[derive(Debug, Default)]
pub(super) struct Channel {
    pub(super) profile: Option<Profile>,
    /// Their latest stream, which may have ended.
    pub(super) latest: Option<LiveStream>,
    pub(super) loading: bool,
    pub(super) error: Option<String>,
    pub(super) viewers: Option<u64>,
    pub(super) chat: VecDeque<LiveChat>,
    pub(super) pinned: Option<LiveChat>,
    /// The chat's connection is open.
    pub(super) connected: bool,
    pub(super) past: Vec<PastStream>,
    pub(super) past_cursor: Option<String>,
    pub(super) past_loading: bool,
    pub(super) draft: String,
    pub(super) sending: bool,
    pub(super) send_error: Option<String>,
    /// People the streamer blocked while the page was open.
    pub(super) blocked: HashSet<Did>,
}

impl Channel {
    pub(super) fn did(&self) -> Option<&Did> {
        self.profile.as_ref().map(|p| &p.did)
    }

    /// Whether the stream is on now: its latest stream hasn't ended, and Bluesky says
    /// they are live on Streamplace (the node keeps that status fresh while live).
    pub(super) fn is_live(&self, now: &str) -> bool {
        let streaming = self
            .profile
            .as_ref()
            .and_then(|p| p.status.as_ref())
            .and_then(|s| s.live_now(now))
            .is_some_and(|l| l.service == "Streamplace");
        streaming && self.latest.as_ref().is_some_and(|s| s.ended.is_none())
    }
}

impl Live {
    /// The DID of the streamer whose page was opened as `actor`, once it has loaded.
    pub fn did_of(&self, actor: &str) -> Option<&Did> {
        self.channels.get(actor).and_then(Channel::did)
    }

    /// The name to show for a streamer's page.
    pub fn title(&self, actor: &str) -> String {
        self.channels
            .get(actor)
            .and_then(|c| c.profile.as_ref())
            .map_or_else(|| actor.to_string(), |p| p.name().to_string())
    }

    /// Whether `actor`'s page shows them live now (`now` as an RFC 3339 time).
    pub fn is_live(&self, actor: &str, now: &str) -> bool {
        self.channels.get(actor).is_some_and(|c| c.is_live(now))
    }

    /// Forgets everything, as when switching accounts.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
