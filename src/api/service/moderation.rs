//! The call behind reporting.

use crate::api::{ApiError, Client, ReportReason, ReportSubject};

/// Sends reports to Bluesky's moderation team. The call returns a future that can
/// move to another thread, so it can run as an iced task.
pub trait ModerationApi: Clone + Send + Sync + 'static {
    fn report(
        &self,
        subject: &ReportSubject,
        reason: ReportReason,
        details: &str,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
}

impl ModerationApi for Client {
    fn report(
        &self,
        subject: &ReportSubject,
        reason: ReportReason,
        details: &str,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::report(self, subject, reason, details)
    }
}
