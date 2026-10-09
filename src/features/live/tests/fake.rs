//! A fake Streamplace and Bluesky for the Live feature's tests.

use crate::api::ApiError;
use crate::api::service::{Channel, LiveApi};
use crate::model::{AtUri, Author, Did, LiveStream, Page, PastStream};

/// Answers every call with "not in the fake": the tests hand the feature its answers
/// as messages themselves.
#[derive(Clone)]
pub struct Fake;

impl Fake {
    fn note<T>(call: String) -> std::future::Ready<Result<T, ApiError>> {
        std::future::ready(Err(ApiError::Other(format!("{call} is not in the fake"))))
    }
}

impl LiveApi for Fake {
    fn live_now(
        &self,
    ) -> impl Future<Output = Result<(Vec<LiveStream>, Vec<Author>), ApiError>> + Send {
        Self::note("live now".into())
    }
    fn channel(&self, actor: &str) -> impl Future<Output = Result<Channel, ApiError>> + Send {
        Self::note(format!("channel {actor}"))
    }
    fn past(
        &self,
        actor: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<PastStream>, ApiError>> + Send {
        Self::note(format!("past {actor} {}", cursor.unwrap_or_default()))
    }
    fn send(
        &self,
        streamer: &Did,
        text: &str,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Self::note(format!("send {streamer} {text}"))
    }
}
