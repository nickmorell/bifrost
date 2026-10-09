//! How far you have been told about.

use crate::model::Alerts as Kinds;
use chrono::{DateTime, Utc};

/// The newest notification and message already seen or shown. Nothing is told until
/// the app says where to start, so what was there before is never announced.
#[derive(Debug, Default)]
pub struct Alerts {
    pub(super) notifications: Option<DateTime<Utc>>,
    pub(super) messages: Option<DateTime<Utc>>,
}

impl Alerts {
    /// Whether [`Msg::Check`](super::Msg::Check) fetches your notifications and your
    /// chats, with these kinds turned on.
    pub fn fetches(&self, kinds: &Kinds) -> (bool, bool) {
        (
            self.notifications.is_some() && kinds.any_notifications(),
            self.messages.is_some() && kinds.messages,
        )
    }

    /// Whether `at` (an RFC 3339 time) is newer than `since`. A time that can't be read
    /// is never new, so a bad one can't announce something twice.
    pub(super) fn newer(since: Option<DateTime<Utc>>, at: &str) -> Option<DateTime<Utc>> {
        let at = DateTime::parse_from_rfc3339(at).ok()?.with_timezone(&Utc);
        since.filter(|s| at > *s).map(|_| at)
    }
}
