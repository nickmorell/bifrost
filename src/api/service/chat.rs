//! The calls behind direct messages.

use crate::api::{ApiError, Client};
use crate::model::{AllowIncoming, Author, ChatMessage, Convo, Did, Page, StrongRef};

/// Chats and their messages, and who may message you. Each call returns a future that
/// can move to another thread, so it can run as an iced task.
pub trait ChatApi: Clone + Send + Sync + 'static {
    /// Your chats, newest first, or the requests waiting to be accepted.
    fn convos(
        &self,
        requests: bool,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<Convo>, ApiError>> + Send;
    /// A page of a chat's messages, newest first.
    fn messages(
        &self,
        convo: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<ChatMessage>, ApiError>> + Send;
    /// The chat with exactly these people, made if there is none.
    fn convo_for(&self, members: &[Did]) -> impl Future<Output = Result<Convo, ApiError>> + Send;
    fn convo(&self, id: &str) -> impl Future<Output = Result<Convo, ApiError>> + Send;
    /// Sends a message, optionally sharing a post, and returns it as sent.
    fn send_message(
        &self,
        convo: &str,
        text: &str,
        post: Option<StrongRef>,
    ) -> impl Future<Output = Result<ChatMessage, ApiError>> + Send;
    fn accept_convo(&self, id: &str) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn leave_convo(&self, id: &str) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn mute_convo(&self, id: &str, mute: bool)
    -> impl Future<Output = Result<(), ApiError>> + Send;
    fn mark_convo_read(&self, id: &str) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// Deletes a message for you only.
    fn delete_message(
        &self,
        convo: &str,
        message: &str,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// Adds or removes your emoji reaction on a message.
    fn react(
        &self,
        convo: &str,
        message: &str,
        value: &str,
        add: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// Who may start a chat with you.
    fn allow_incoming(&self) -> impl Future<Output = Result<AllowIncoming, ApiError>> + Send;
    fn set_allow_incoming(
        &self,
        allow: AllowIncoming,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// People whose name or handle starts with `q`.
    fn typeahead(&self, q: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send;
}

impl ChatApi for Client {
    fn convos(
        &self,
        requests: bool,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<Convo>, ApiError>> + Send {
        Client::convos(self, requests, cursor)
    }
    fn messages(
        &self,
        convo: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<ChatMessage>, ApiError>> + Send {
        Client::messages(self, convo, cursor)
    }
    fn convo_for(&self, members: &[Did]) -> impl Future<Output = Result<Convo, ApiError>> + Send {
        Client::convo_for(self, members)
    }
    fn convo(&self, id: &str) -> impl Future<Output = Result<Convo, ApiError>> + Send {
        Client::convo(self, id)
    }
    fn send_message(
        &self,
        convo: &str,
        text: &str,
        post: Option<StrongRef>,
    ) -> impl Future<Output = Result<ChatMessage, ApiError>> + Send {
        Client::send_message(self, convo, text, post)
    }
    fn accept_convo(&self, id: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::accept_convo(self, id)
    }
    fn leave_convo(&self, id: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::leave_convo(self, id)
    }
    fn mute_convo(
        &self,
        id: &str,
        mute: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::mute_convo(self, id, mute)
    }
    fn mark_convo_read(&self, id: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::mark_convo_read(self, id)
    }
    fn delete_message(
        &self,
        convo: &str,
        message: &str,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::delete_message(self, convo, message)
    }
    fn react(
        &self,
        convo: &str,
        message: &str,
        value: &str,
        add: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::react(self, convo, message, value, add)
    }
    fn allow_incoming(&self) -> impl Future<Output = Result<AllowIncoming, ApiError>> + Send {
        Client::allow_incoming(self)
    }
    fn set_allow_incoming(
        &self,
        allow: AllowIncoming,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::set_allow_incoming(self, allow)
    }
    fn typeahead(&self, q: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        Client::typeahead(self, q)
    }
}
