//! A fake Bluesky for the thread feature's tests.

use crate::api::ApiError;
use crate::api::service::ThreadApi;
use crate::model::{AtUri, ThreadNode};
use std::sync::{Arc, Mutex};

/// Bluesky as far as these tests need it: it notes every call and finds nothing.
#[derive(Clone, Default)]
pub struct Fake {
    pub calls: Arc<Mutex<Vec<String>>>,
}

impl ThreadApi for Fake {
    fn thread(&self, uri: &AtUri) -> impl Future<Output = Result<ThreadNode, ApiError>> + Send {
        self.calls.lock().unwrap().push(format!("thread {uri}"));
        async { Ok(ThreadNode::NotFound) }
    }
}
