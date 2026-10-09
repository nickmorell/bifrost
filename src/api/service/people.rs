//! The call behind lists of accounts.

use crate::api::{ApiError, Client};
use crate::model::{Author, Page, PeopleKind};

/// Someone's followers or follows, or your muted or blocked accounts. The call returns
/// a future that can move to another thread, so it can run as an iced task.
pub trait PeopleApi: Clone + Send + Sync + 'static {
    /// A page of the accounts in the list `kind` of `actor`.
    fn people(
        &self,
        actor: &str,
        kind: PeopleKind,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<Author>, ApiError>> + Send;
}

impl PeopleApi for Client {
    fn people(
        &self,
        actor: &str,
        kind: PeopleKind,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<Author>, ApiError>> + Send {
        // The call is large; boxing keeps every task that awaits it small.
        Box::pin(Client::people(self, actor, kind, cursor))
    }
}
