//! What moderation is told, and what it asks of the app.

use crate::api::{ApiError, ReportReason, ReportSubject};
use crate::model::{PeopleKind, PrefEdit};

#[derive(Debug, Clone)]
pub enum Msg {
    /// Change a filter or unmute a word.
    Edit(PrefEdit),
    /// The muted word being typed.
    WordInput(String),
    AddWord,
    /// Show your muted or blocked accounts.
    People(PeopleKind),
    /// Start reporting a post or an account.
    Report(ReportSubject),
    Reason(ReportReason),
    Details(String),
    Submit,
    Cancel,
    Sent(Result<(), ApiError>),
}

/// What the app has to do for moderation.
#[derive(Debug)]
pub enum Event {
    /// Change your preferences, and save them.
    Edit(PrefEdit),
    /// Show your muted or blocked accounts.
    People(PeopleKind),
    Toast(String),
    /// Sending a report failed.
    Failed(ApiError),
}
