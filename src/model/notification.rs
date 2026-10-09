//! Notifications.

use super::{AtUri, Author, Label};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub uri: AtUri,
    pub cid: String,
    pub author: Author,
    /// like, repost, follow, mention, reply, quote, starterpack-joined, ...
    pub reason: String,
    pub reason_subject: Option<AtUri>,
    #[serde(default)]
    pub is_read: bool,
    #[serde(default)]
    pub indexed_at: String,
    #[serde(default)]
    pub labels: Vec<Label>,
}

impl Notification {
    /// The post this notification is about, if any: the notifying post itself for
    /// replies, mentions and quotes, the liked or reposted post otherwise.
    pub fn subject_uri(&self) -> Option<&AtUri> {
        match self.reason.as_str() {
            "reply" | "mention" | "quote" => Some(&self.uri),
            "like" | "repost" | "like-via-repost" | "repost-via-repost" => {
                self.reason_subject.as_ref()
            }
            _ => None,
        }
        .filter(|u| u.contains("/app.bsky.feed.post/"))
    }
}

#[cfg(test)]
mod tests {
    use crate::model::*;
    use serde_json::json;

    #[test]
    fn notification_subject() {
        let n: Notification = serde_json::from_value(json!({
            "uri": "at://did:plc:b/app.bsky.feed.like/1", "cid": "c",
            "author": {"did": "did:plc:b", "handle": "b.test"},
            "reason": "like", "reasonSubject": "at://did:plc:a/app.bsky.feed.post/1",
            "isRead": false, "indexedAt": "", "record": {}
        }))
        .unwrap();
        assert_eq!(
            n.subject_uri().map(AtUri::as_str),
            Some("at://did:plc:a/app.bsky.feed.post/1")
        );
    }
}
