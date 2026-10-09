//! A fake Bluesky for the chat feature's tests. The tests check what `update` does
//! with each message, so the calls are never run and the fake answers nothing.

use crate::api::ApiError;
use crate::api::service::ChatApi;
use crate::model::{AllowIncoming, Author, ChatMessage, Convo, Did, Page, StrongRef};

#[derive(Clone, Default)]
pub struct Fake;

fn none<T>() -> std::future::Ready<Result<T, ApiError>> {
    std::future::ready(Err(ApiError::Other("not in the fake".into())))
}

impl ChatApi for Fake {
    fn convos(
        &self,
        _: bool,
        _: Option<String>,
    ) -> impl Future<Output = Result<Page<Convo>, ApiError>> + Send {
        none()
    }
    fn messages(
        &self,
        _: &str,
        _: Option<String>,
    ) -> impl Future<Output = Result<Page<ChatMessage>, ApiError>> + Send {
        none()
    }
    fn convo_for(&self, _: &[Did]) -> impl Future<Output = Result<Convo, ApiError>> + Send {
        none()
    }
    fn convo(&self, _: &str) -> impl Future<Output = Result<Convo, ApiError>> + Send {
        none()
    }
    fn send_message(
        &self,
        _: &str,
        _: &str,
        _: Option<StrongRef>,
    ) -> impl Future<Output = Result<ChatMessage, ApiError>> + Send {
        none()
    }
    fn accept_convo(&self, _: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        none()
    }
    fn leave_convo(&self, _: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        none()
    }
    fn mute_convo(&self, _: &str, _: bool) -> impl Future<Output = Result<(), ApiError>> + Send {
        none()
    }
    fn mark_convo_read(&self, _: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        none()
    }
    fn delete_message(
        &self,
        _: &str,
        _: &str,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        none()
    }
    fn react(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        none()
    }
    fn allow_incoming(&self) -> impl Future<Output = Result<AllowIncoming, ApiError>> + Send {
        none()
    }
    fn set_allow_incoming(
        &self,
        _: AllowIncoming,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        none()
    }
    fn typeahead(&self, _: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        none()
    }
}
