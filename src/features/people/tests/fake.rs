//! A fake Bluesky for the people feature's tests.

use crate::api::ApiError;
use crate::api::service::PeopleApi;
use crate::model::{Author, Page, PeopleKind};

/// Answers every call with an empty page. Calls run only when their task does, so
/// the tests check the tasks started rather than the calls.
#[derive(Clone)]
pub struct Fake;

impl PeopleApi for Fake {
    async fn people(
        &self,
        _actor: &str,
        _kind: PeopleKind,
        _cursor: Option<String>,
    ) -> Result<Page<Author>, ApiError> {
        Ok(Page {
            items: Vec::new(),
            cursor: None,
        })
    }
}
