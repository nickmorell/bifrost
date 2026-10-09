//! The muted word being typed, and the report being written.

use crate::api::{ReportReason, ReportSubject};

#[derive(Debug, Default)]
pub struct Moderation {
    /// The muted word being typed on the moderation screen.
    pub(super) muted_word: String,
    pub(super) report: Option<Report>,
}

/// A report being written.
#[derive(Debug, Clone)]
pub(super) struct Report {
    pub(super) subject: ReportSubject,
    pub(super) reason: Option<ReportReason>,
    pub(super) details: String,
    pub(super) sending: bool,
}

impl Moderation {
    /// Whether the report dialog is open.
    pub fn reporting(&self) -> bool {
        self.report.is_some()
    }
}
