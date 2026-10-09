//! Which kinds of activity BiFrost tells you about through the operating system's
//! notifications while its window is in the background.

use serde::{Deserialize, Serialize};

/// A kind of activity that can pop up as an OS notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    /// Mentions and quotes of your posts.
    Mentions,
    Replies,
    Likes,
    Reposts,
    Follows,
    /// Direct messages.
    Messages,
}

impl AlertKind {
    pub const ALL: [AlertKind; 6] = [
        AlertKind::Mentions,
        AlertKind::Replies,
        AlertKind::Likes,
        AlertKind::Reposts,
        AlertKind::Follows,
        AlertKind::Messages,
    ];

    /// The kind a notification's `reason` belongs to, if it is one BiFrost alerts on.
    pub fn of_reason(reason: &str) -> Option<AlertKind> {
        Some(match reason {
            "mention" | "quote" => AlertKind::Mentions,
            "reply" => AlertKind::Replies,
            "like" | "like-via-repost" => AlertKind::Likes,
            "repost" | "repost-via-repost" => AlertKind::Reposts,
            "follow" => AlertKind::Follows,
            _ => return None,
        })
    }
}

/// Which kinds are switched on. Mentions and direct messages are on to begin with;
/// the rest would be noise for most people.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Alerts {
    pub mentions: bool,
    pub replies: bool,
    pub likes: bool,
    pub reposts: bool,
    pub follows: bool,
    pub messages: bool,
    /// Whether a direct message's notice shows what it says, not just who sent it.
    /// Off to begin with, so a message can't be read off a locked or shared screen.
    pub message_text: bool,
}

impl Default for Alerts {
    fn default() -> Self {
        Self {
            mentions: true,
            replies: false,
            likes: false,
            reposts: false,
            follows: false,
            messages: true,
            message_text: false,
        }
    }
}

impl Alerts {
    pub fn is_on(&self, kind: AlertKind) -> bool {
        *self.slot(kind)
    }

    pub fn set(&mut self, kind: AlertKind, on: bool) {
        *self.slot_mut(kind) = on;
    }

    /// Whether any notification kind (anything but direct messages) is on.
    pub fn any_notifications(&self) -> bool {
        self.mentions || self.replies || self.likes || self.reposts || self.follows
    }

    fn slot(&self, kind: AlertKind) -> &bool {
        match kind {
            AlertKind::Mentions => &self.mentions,
            AlertKind::Replies => &self.replies,
            AlertKind::Likes => &self.likes,
            AlertKind::Reposts => &self.reposts,
            AlertKind::Follows => &self.follows,
            AlertKind::Messages => &self.messages,
        }
    }

    fn slot_mut(&mut self, kind: AlertKind) -> &mut bool {
        match kind {
            AlertKind::Mentions => &mut self.mentions,
            AlertKind::Replies => &mut self.replies,
            AlertKind::Likes => &mut self.likes,
            AlertKind::Reposts => &mut self.reposts,
            AlertKind::Follows => &mut self.follows,
            AlertKind::Messages => &mut self.messages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mentions_and_messages_are_on_by_default() {
        let a = Alerts::default();
        let on: Vec<AlertKind> = AlertKind::ALL.into_iter().filter(|k| a.is_on(*k)).collect();
        assert_eq!(on, [AlertKind::Mentions, AlertKind::Messages]);
    }

    #[test]
    fn missing_fields_keep_their_defaults() {
        let a: Alerts = serde_json::from_str(r#"{"likes":true}"#).unwrap();
        assert!(a.likes && a.mentions && a.messages && !a.follows);
    }

    #[test]
    fn quotes_count_as_mentions_and_unknown_reasons_as_nothing() {
        assert_eq!(AlertKind::of_reason("quote"), Some(AlertKind::Mentions));
        assert_eq!(AlertKind::of_reason("starterpack-joined"), None);
    }

    #[test]
    fn set_switches_one_kind() {
        let mut a = Alerts::default();
        a.set(AlertKind::Follows, true);
        a.set(AlertKind::Mentions, false);
        assert!(a.follows && !a.mentions && a.any_notifications());
    }
}
