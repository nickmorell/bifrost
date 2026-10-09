//! The thread screen: one post with the posts above it, joined by a line, and its
//! replies nested below, kept for every thread opened.
//!
//! The app keeps a [`Threads`], passes it every [`Msg`] with [`Threads::update`], and
//! does what the returned [`Event`]s ask. Bluesky is reached through
//! [`ThreadApi`](crate::api::service::ThreadApi). The posts are drawn by the app, so the
//! screen is drawn through [`View`] with a [`Host`].

mod message;
mod state;
mod tree;
mod update;
mod view;

pub use message::{Event, Msg};
pub use state::Threads;
pub use view::{Host, Shape, View};

type Out = crate::features::Update<Msg, Event>;

#[cfg(test)]
mod tests;
