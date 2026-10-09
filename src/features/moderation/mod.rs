//! Moderation: the screen with content filters, muted words and links to muted and
//! blocked accounts, and the dialog for reporting a post or an account.
//!
//! The filters and muted words are part of your Bluesky preferences, which the app
//! keeps, so the screen is handed them and asks for changes as events. The app keeps
//! a [`Moderation`], passes it every [`Msg`] with [`Moderation::update`], and does
//! what the returned [`Event`]s ask. Reports go through
//! [`ModerationApi`](crate::api::service::ModerationApi).

mod message;
mod state;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use message::{Event, Msg};
pub use state::Moderation;
pub use view::Screen;

type Out = crate::features::Update<Msg, Event>;
