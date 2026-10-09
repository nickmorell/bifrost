//! What the chat feature keeps: the chat lists, each chat's messages, and its dialogs.

use crate::model::{AllowIncoming, Author, ChatMessage, Convo, Did, Reaction, Sender, StrongRef};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Bluesky's limit on a message, in graphemes.
pub(super) const MAX_MESSAGE: usize = 1000;
/// How often an open chat checks for new messages.
pub const OPEN_POLL: Duration = Duration::from_secs(4);
/// The open chat once nothing has happened in it for a minute.
pub const QUIET_POLL: Duration = Duration::from_secs(15);
/// How often the chat list and the unread count are refreshed.
pub const LIST_POLL: Duration = Duration::from_secs(30);
/// The most people a chat can have, you included.
pub(super) const MAX_MEMBERS: usize = 10;
/// Quick reactions offered on a message.
pub(super) const REACTIONS: [&str; 6] = ["❤️", "😂", "👍", "😮", "😢", "🎉"];
/// Load older messages when scrolled within this far of the top.
pub(super) const LOAD_OLDER_AT: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetch {
    First,
    Older,
    Poll,
}

#[derive(Debug, Default)]
pub struct ConvoList {
    pub ids: Vec<String>,
    pub cursor: Option<String>,
    pub loading: bool,
    pub loaded: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct ChatThread {
    /// Oldest first.
    pub messages: Vec<ChatMessage>,
    pub cursor: Option<String>,
    pub loading: bool,
    pub loaded: bool,
    pub polling: bool,
    pub error: Option<String>,
    pub draft: String,
    pub sending: bool,
}

#[derive(Debug, Default)]
pub struct NewChat {
    pub query: String,
    pub results: Vec<Author>,
    pub picked: Vec<Author>,
    pub seq: u64,
    pub starting: bool,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct Share {
    pub post: StrongRef,
    pub text: String,
    pub sending: Option<String>,
    pub sent: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct Chats {
    /// When the open chat last had a message come or go, so polling can slow down
    /// once it goes quiet.
    pub active_at: Option<Instant>,
    pub convos: HashMap<String, Convo>,
    pub accepted: ConvoList,
    pub requests: ConvoList,
    pub showing_requests: bool,
    pub threads: HashMap<String, ChatThread>,
    /// This sign-in was refused access to chats.
    pub no_access: bool,
    pub unread: u64,
    pub new_chat: Option<NewChat>,
    pub share: Option<Share>,
    pub allow: Option<AllowIncoming>,
    /// The message whose menu is open.
    pub menu: Option<String>,
    pub confirm_leave: Option<String>,
    /// Opening a chat with someone from their profile.
    pub opening: bool,
}

impl Chats {
    /// The title bar's text for a chat.
    pub fn title(&self, id: &str, me: &Did) -> String {
        match self.convos.get(id) {
            Some(c) => c.title(me),
            None => "Chat".into(),
        }
    }

    pub(super) fn list(&mut self, requests: bool) -> &mut ConvoList {
        if requests {
            &mut self.requests
        } else {
            &mut self.accepted
        }
    }

    /// Unread messages in accepted, unmuted chats, plus requests with anything unread.
    pub(super) fn count_unread(&mut self) {
        let accepted: u64 = self
            .accepted
            .ids
            .iter()
            .filter_map(|id| self.convos.get(id))
            .filter(|c| !c.muted && !c.is_request())
            .map(|c| c.unread_count)
            .sum();
        let requests = self
            .requests
            .ids
            .iter()
            .filter_map(|id| self.convos.get(id))
            .filter(|c| c.is_request() && c.unread_count > 0)
            .count() as u64;
        self.unread = accepted + requests;
    }

    pub(super) fn forget(&mut self, id: &str) {
        self.convos.remove(id);
        self.threads.remove(id);
        self.accepted.ids.retain(|i| i != id);
        self.requests.ids.retain(|i| i != id);
        self.count_unread();
    }

    /// Moves a chat to the top of the right list, as when a message arrives.
    pub(super) fn bump(&mut self, convo: Convo) {
        let id = convo.id.clone();
        let requests = convo.is_request();
        self.accepted.ids.retain(|i| *i != id);
        self.requests.ids.retain(|i| *i != id);
        self.list(requests).ids.insert(0, id.clone());
        self.convos.insert(id, convo);
        self.count_unread();
    }

    /// Shows or hides your reaction on a message.
    pub(super) fn set_reaction(
        &mut self,
        me: &Did,
        convo: &str,
        message: &str,
        value: &str,
        on: bool,
    ) {
        let Some(m) = self
            .threads
            .get_mut(convo)
            .and_then(|t| t.messages.iter_mut().find(|m| m.id == message))
        else {
            return;
        };
        m.reactions
            .retain(|r| !(r.value == value && r.sender.did == *me));
        if on {
            m.reactions.push(Reaction {
                value: value.into(),
                sender: Sender { did: me.clone() },
            });
        }
    }

    /// The pictures of a chat's members.
    pub(super) fn avatars(&self, id: &str) -> Vec<String> {
        self.convos
            .get(id)
            .map(|c| c.members.iter().filter_map(|m| m.avatar.clone()).collect())
            .unwrap_or_default()
    }
}

/// Adds a page of messages (newest first, as the server sends them) to a chat's.
pub(super) fn merge(messages: &mut Vec<ChatMessage>, page: Vec<ChatMessage>) -> bool {
    let mut added = false;
    for m in page.into_iter().rev() {
        if let Some(x) = messages.iter_mut().find(|x| x.id == m.id) {
            *x = m;
        } else {
            messages.push(m);
            added = true;
        }
    }
    messages.sort_by(|a, b| a.sent_at.cmp(&b.sent_at).then_with(|| a.id.cmp(&b.id)));
    added
}
