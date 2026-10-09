//! Direct messages: conversations, messages and reactions.

use super::{Author, Did, Embed, PostRecord};
use serde::Deserialize;
use serde_json::Value;
use std::fmt::Display;

/// A conversation with one or more people.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Convo {
    pub id: String,
    #[serde(default)]
    pub rev: String,
    /// Everyone in it, you included.
    #[serde(default)]
    pub members: Vec<Author>,
    #[serde(default, deserialize_with = "lenient_message")]
    pub last_message: Option<ChatMessage>,
    #[serde(default)]
    pub muted: bool,
    /// "request" until you accept it, then "accepted".
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub unread_count: u64,
}

impl Convo {
    /// A chat someone started with you that you have not accepted yet.
    pub fn is_request(&self) -> bool {
        self.status.as_deref() == Some("request")
    }

    /// Everyone except you.
    pub fn others<'a>(&'a self, me: &'a str) -> impl Iterator<Item = &'a Author> + 'a {
        self.members.iter().filter(move |m| m.did != me)
    }

    pub fn is_group(&self, me: &str) -> bool {
        self.others(me).count() > 1
    }

    /// The chat's name: the other person, or everyone in a group.
    pub fn title(&self, me: &str) -> String {
        let names: Vec<&str> = self.others(me).map(Author::name).collect();
        match names.len() {
            0 => "Just you".into(),
            1..=3 => names.join(", "),
            n => format!("{}, {} and {} others", names[0], names[1], n - 2),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    #[serde(default)]
    pub rev: String,
    pub sender: Sender,
    #[serde(default)]
    pub sent_at: String,
    /// `chat.bsky.convo.defs#messageView` or `#deletedMessageView`.
    #[serde(rename = "$type", default)]
    pub kind: String,
    /// The text and its links and mentions.
    #[serde(flatten)]
    pub body: PostRecord,
    #[serde(default)]
    pub embed: Option<Embed>,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
}

impl ChatMessage {
    /// Whether `next` belongs with this message: the same person sent it minutes apart.
    pub fn same_run(&self, next: &ChatMessage) -> bool {
        let at = |m: &ChatMessage| chrono::DateTime::parse_from_rfc3339(&m.sent_at).ok();
        self.sender.did == next.sender.did
            && match (at(self), at(next)) {
                (Some(x), Some(y)) => (y - x).num_minutes().abs() < 5,
                _ => false,
            }
    }

    pub fn is_deleted(&self) -> bool {
        self.kind.ends_with("deletedMessageView")
    }

    /// One line for the chat list.
    pub fn preview(&self) -> String {
        if self.is_deleted() {
            return "Message deleted".into();
        }
        let text = self
            .body
            .text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        match (text.is_empty(), &self.embed) {
            (true, Some(_)) => "Shared a post".into(),
            _ => text,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sender {
    pub did: Did,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    pub value: String,
    pub sender: Sender,
}

/// Who can start a chat with you.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowIncoming {
    All,
    Following,
    None,
}

impl AllowIncoming {
    pub const ALL: [AllowIncoming; 3] = [Self::All, Self::Following, Self::None];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Following => "following",
            Self::None => "none",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "all" => Self::All,
            "none" => Self::None,
            // Bluesky's default.
            _ => Self::Following,
        }
    }
}

impl Display for AllowIncoming {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::All => "Everyone",
            Self::Following => "People you follow",
            Self::None => "No one",
        })
    }
}

fn lenient_message<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<ChatMessage>, D::Error> {
    let v = Value::deserialize(d)?;
    Ok(serde_json::from_value(v).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn msg(id: &str, sender: &str, at: &str) -> ChatMessage {
        serde_json::from_value(json!({
            "$type": "chat.bsky.convo.defs#messageView",
            "id": id, "rev": id, "sender": {"did": sender}, "sentAt": at, "text": id
        }))
        .unwrap()
    }

    #[test]
    fn runs_break_on_sender_or_time() {
        let a = msg("a", "x", "2026-10-05T10:00:00Z");
        assert!(a.same_run(&msg("b", "x", "2026-10-05T10:03:00Z")));
        assert!(!a.same_run(&msg("b", "x", "2026-10-05T10:09:00Z")));
        assert!(!a.same_run(&msg("b", "y", "2026-10-05T10:01:00Z")));
    }
}
