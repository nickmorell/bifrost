//! Chats: the chat list and requests, one chat with its messages and reactions,
//! starting a chat or group, sending a post in a chat, and who can message you.
//!
//! The app keeps a [`Chats`], passes it every [`Msg`] with [`Chats::update`], and
//! does what the returned [`Event`]s ask. Bluesky is reached through
//! [`ChatApi`](crate::api::service::ChatApi). The views are drawn through [`View`],
//! which needs a [`Host`] because a chat message can carry a post the app draws.

mod load;
mod message;
mod people;
mod state;
mod update;
mod view;

pub use message::{Ctx, Event, Msg};
pub use state::{
    ChatThread, Chats, ConvoList, Fetch, LIST_POLL, NewChat, OPEN_POLL, QUIET_POLL, Share,
};
pub use view::{Host, View};

type Out = crate::features::Update<Msg, Event>;

#[cfg(test)]
mod tests;
