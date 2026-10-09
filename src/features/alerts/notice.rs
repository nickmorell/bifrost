//! The words of each notice.

use super::message::Notice;
use crate::model::{Author, ChatMessage, Notification};

/// More new things than this at once are summed up in one notice.
pub(super) const MAX_EACH: usize = 3;

/// Whether a notification's notice carries its post's text.
pub(super) fn needs_text(n: &Notification) -> bool {
    matches!(n.reason.as_str(), "mention" | "reply" | "quote")
}

pub(super) fn for_notification(n: &Notification, text: Option<&str>) -> Notice {
    let who = n.author.name();
    let title = match n.reason.as_str() {
        "mention" => format!("{who} mentioned you"),
        "quote" => format!("{who} quoted your post"),
        "reply" => format!("{who} replied to you"),
        "like" => format!("{who} liked your post"),
        "like-via-repost" => format!("{who} liked your repost"),
        "repost" => format!("{who} reposted your post"),
        "repost-via-repost" => format!("{who} reposted your repost"),
        "follow" => format!("{who} followed you"),
        other => format!("{who}: {other}"),
    };
    Notice {
        title,
        body: text.unwrap_or_default().to_string(),
    }
}

/// A direct message's notice: who sent it, and what it says only when `text` is on.
pub(super) fn for_message(from: Option<&Author>, m: &ChatMessage, text: bool) -> Notice {
    let who = from.map_or("Someone", Author::name);
    if text {
        Notice {
            title: who.to_string(),
            body: m.body.text.clone(),
        }
    } else {
        Notice {
            title: "New message".into(),
            body: format!("From {who}"),
        }
    }
}

/// One notice for many new things.
pub(super) fn summary(count: usize, what: &str) -> Notice {
    Notice {
        title: "BiFrost".into(),
        body: format!("{count} new {what}"),
    }
}
