//! A fake Bluesky for the alerts feature's tests. The tests hand results to `update`
//! themselves, so the fake only has to exist.

use crate::api::ApiError;
use crate::api::service::AlertsApi;
use crate::model::{AtUri, Convo, Notification, Page, PostView};

#[derive(Clone, Copy, Default)]
pub struct Fake;

impl AlertsApi for Fake {
    async fn notifications(&self) -> Result<Page<Notification>, ApiError> {
        Ok(Page {
            items: Vec::new(),
            cursor: None,
        })
    }
    async fn posts(&self, _uris: &[AtUri]) -> Result<Vec<PostView>, ApiError> {
        Ok(Vec::new())
    }
    async fn convos(&self) -> Result<Page<Convo>, ApiError> {
        Ok(Page {
            items: Vec::new(),
            cursor: None,
        })
    }
}
