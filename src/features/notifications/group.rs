//! Grouping notifications the way the official app does: "Alice and 12 others liked
//! your post" instead of thirteen rows.

use crate::model::Notification;

/// Notifications that can be folded together.
const GROUPED: &[&str] = &[
    "like",
    "repost",
    "follow",
    "like-via-repost",
    "repost-via-repost",
];

/// Likes of the same post this far apart still share a row.
const WINDOW_SECS: i64 = 48 * 3600;

/// One row in the notifications list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Indices into the notification list, newest first.
    pub members: Vec<usize>,
}

/// Which notifications the list shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    /// Replies, mentions and quotes: the ones that need an answer.
    Mentions,
}

impl Filter {
    pub fn shows(self, n: &Notification) -> bool {
        match self {
            Filter::All => true,
            Filter::Mentions => matches!(n.reason.as_str(), "reply" | "mention" | "quote"),
        }
    }
}

fn time(n: &Notification) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(&n.indexed_at)
        .ok()
        .map(|t| t.timestamp())
}

/// Groups `items` (newest first) into rows, keeping the order of each row's newest
/// notification.
pub fn group(items: &[Notification], filter: Filter) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for (i, n) in items.iter().enumerate() {
        if !filter.shows(n) {
            continue;
        }
        let joins = GROUPED.contains(&n.reason.as_str()).then(|| {
            out.iter_mut().find(|g| {
                let first = &items[g.members[0]];
                let close = match (time(first), time(n)) {
                    (Some(a), Some(b)) => (a - b).abs() <= WINDOW_SECS,
                    _ => true,
                };
                first.reason == n.reason && first.reason_subject == n.reason_subject && close
            })
        });
        match joins.flatten() {
            // The same person twice (liked, unliked, liked again) counts once.
            Some(g)
                if g.members
                    .iter()
                    .any(|&m| items[m].author.did == n.author.did) => {}
            Some(g) => g.members.push(i),
            None => out.push(Group { members: vec![i] }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn n(reason: &str, who: &str, subject: Option<&str>, at: &str) -> Notification {
        serde_json::from_value(json!({
            "uri": format!("at://{who}/x/{reason}{at}"), "cid": "c",
            "author": {"did": who, "handle": "h.test"},
            "reason": reason, "reasonSubject": subject,
            "isRead": false, "indexedAt": at, "record": {}
        }))
        .unwrap()
    }

    #[test]
    fn likes_of_one_post_share_a_row() {
        let items = vec![
            n("like", "did:a", Some("at://p/1"), "2026-10-04T10:00:00Z"),
            n("reply", "did:c", None, "2026-10-04T09:30:00Z"),
            n("like", "did:b", Some("at://p/1"), "2026-10-04T09:00:00Z"),
            n("like", "did:a", Some("at://p/1"), "2026-10-04T08:00:00Z"),
            n("like", "did:d", Some("at://p/2"), "2026-10-04T07:00:00Z"),
            n("follow", "did:e", None, "2026-10-04T06:00:00Z"),
            n("follow", "did:f", None, "2026-10-04T05:00:00Z"),
            n("like", "did:g", Some("at://p/1"), "2026-09-01T05:00:00Z"),
        ];
        let rows: Vec<Vec<usize>> = group(&items, Filter::All)
            .into_iter()
            .map(|g| g.members)
            .collect();
        assert_eq!(
            rows,
            vec![vec![0, 2], vec![1], vec![4], vec![5, 6], vec![7]]
        );
        let mentions = group(&items, Filter::Mentions);
        assert_eq!(mentions, vec![Group { members: vec![1] }]);
    }
}
