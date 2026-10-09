//! Live streams on Streamplace: who is live now, each streamer's page with their
//! stream, past streams and live chat, and sending chat messages.
//!
//! The app keeps a [`Live`], passes it every [`Msg`] with [`Live::update`], and does
//! what the returned [`Event`]s ask: it plays streams in its video player, and feeds
//! the chat of the open stream in as [`Msg::Happened`] from Streamplace's websocket
//! (`api::streamplace::events`). Streams and profiles come through
//! [`LiveApi`](crate::api::service::LiveApi).

mod message;
mod state;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use message::{Event, Msg};
pub use state::{Live, MAX_MESSAGE};
pub use view::{Host, View};

type Out = crate::features::Update<Msg, Event>;

/// The key the app's video player knows a streamer's live stream by.
pub fn player_key(streamer: &crate::model::Did) -> String {
    format!("live:{streamer}")
}
