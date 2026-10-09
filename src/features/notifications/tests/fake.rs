//! A fake Bluesky for the notifications feature's tests.

use crate::api::ApiError;
use crate::api::service::NotificationsApi;
use crate::model::{AtUri, Notification, Page, PostView};
use std::sync::{Arc, Mutex};

/// Bluesky as far as these tests need it: it notes every call and answers from what
/// it was given.
#[derive(Clone, Default)]
pub struct Fake {
    pub calls: Arc<Mutex<Vec<String>>>,
    pub page: Vec<Notification>,
    pub posts: Vec<PostView>,
}

impl Fake {
    fn note(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl NotificationsApi for Fake {
    fn notifications(
        &self,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<Notification>, ApiError>> + Send {
        self.note(format!("notifications {cursor:?}"));
        let items = self.page.clone();
        async move {
            Ok(Page {
                items,
                cursor: Some("next".into()),
            })
        }
    }
    fn posts(
        &self,
        uris: &[AtUri],
    ) -> impl Future<Output = Result<Vec<PostView>, ApiError>> + Send {
        let list: Vec<&str> = uris.iter().map(AtUri::as_str).collect();
        self.note(format!("posts {}", list.join(" ")));
        let posts: Vec<PostView> = self
            .posts
            .iter()
            .filter(|p| uris.contains(&p.uri))
            .cloned()
            .collect();
        async move { Ok(posts) }
    }
    fn unread_count(&self) -> impl Future<Output = Result<u64, ApiError>> + Send {
        self.note("unread".into());
        async { Ok(0) }
    }
    fn mark_notifications_seen(&self) -> impl Future<Output = Result<(), ApiError>> + Send {
        self.note("seen".into());
        async { Ok(()) }
    }
}
