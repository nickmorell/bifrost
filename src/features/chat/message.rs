//! What the chat feature takes in ([`Msg`]) and what it asks of the app ([`Event`]).

use super::state::Fetch;
use crate::api::ApiError;
use crate::model::{AllowIncoming, Author, ChatMessage, Convo, Did, Page, StrongRef};
use iced::widget::scrollable::Viewport;

#[derive(Debug, Clone)]
pub enum Msg {
    Open(String),
    Tab(bool),
    /// The first page of a list (or more of it, when appending).
    Listed(bool, bool, Result<Page<Convo>, ApiError>),
    MoreConvos,
    /// Refresh the lists and the unread count.
    Tick,
    /// Check the open chat for new messages.
    Poll,
    Messages(String, Fetch, Result<Page<ChatMessage>, ApiError>),
    Scrolled(String, Viewport),
    Draft(String, String),
    Send(String),
    Sent(String, Result<Box<ChatMessage>, ApiError>),
    Accept(String),
    Accepted(String, Result<(), ApiError>),
    Leave(String),
    Left(String, Result<(), ApiError>),
    Mute(String, bool),
    Muted(String, bool, Result<(), ApiError>),
    /// Open or close a message's menu.
    Menu(Option<String>),
    React(String, String, String, bool),
    /// A reaction was saved, or not; undone on failure.
    Reacted(String, String, String, bool, Result<(), ApiError>),
    Delete(String, String),
    Deleted(String, String, Result<(), ApiError>),
    Read(Result<(), ApiError>),

    /// Start a chat with this person, or open the one you have.
    MessagePerson(Did),
    Started(Result<Box<Convo>, ApiError>),
    NewChat,
    NewQuery(String),
    NewSuggest(u64),
    NewResults(u64, Result<Vec<Author>, ApiError>),
    NewPick(Author),
    NewUnpick(Did),
    NewStart,
    CloseNew,

    /// Send a post into a chat.
    SharePost(StrongRef),
    ShareText(String),
    ShareTo(String),
    Shared(String, Result<Box<ChatMessage>, ApiError>),
    CloseShare,

    AllowLoaded(Result<AllowIncoming, ApiError>),
    SetAllow(AllowIncoming),
    AllowSaved(AllowIncoming, Result<(), ApiError>),

    /// Pressed in a view; passed on to the app as the [`Event`] of the same name.
    OpenProfile(Did),
    CopyText(String),
    Block(Did),
    Report(Did),
    SignOut,
    OpenUrl(String),
    Refresh,
    Retry,
    SignInAgain,
    /// The chat list scrolled.
    ListScrolled(Viewport),
}

impl Msg {
    /// Whether this is a result saying the sign-in has expired.
    pub(super) fn expired(&self) -> bool {
        let (Msg::Listed(.., Err(e))
        | Msg::Messages(.., Err(e))
        | Msg::Sent(_, Err(e))
        | Msg::Accepted(_, Err(e))
        | Msg::Left(_, Err(e))
        | Msg::Muted(.., Err(e))
        | Msg::Reacted(.., Err(e))
        | Msg::Deleted(.., Err(e))
        | Msg::Read(Err(e))
        | Msg::Started(Err(e))
        | Msg::Shared(_, Err(e))
        | Msg::AllowSaved(_, Err(e))) = self
        else {
            return false;
        };
        matches!(e, ApiError::SessionExpired)
    }
}

/// Something the app has to do for the chat feature, in the order given.
#[derive(Debug)]
pub enum Event {
    /// Show this chat.
    OpenChat(String),
    /// Show the chat list.
    OpenChats,
    /// Leave the chat being shown.
    Back,
    /// The sign-in has expired.
    SessionExpired,
    Toast(String),
    /// Fetch these pictures.
    Images(Vec<String>),
    /// Close any open post menu.
    CloseMenu,
    OpenProfile(Did),
    CopyText(String),
    Block(Did),
    Report(Did),
    SignOut,
    OpenUrl(String),
    Refresh,
    Retry,
    SignInAgain,
    /// The chat list scrolled, for keeping its place.
    ListScrolled(Viewport),
}

/// What the feature needs to know about the app around it.
#[derive(Debug, Clone, Copy)]
pub struct Ctx<'a> {
    pub me: &'a Did,
    /// The chat on screen, if one is.
    pub open: Option<&'a str>,
}
