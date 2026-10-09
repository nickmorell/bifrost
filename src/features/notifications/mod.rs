//! Notifications: the list, grouped the way the official app does, its All and
//! Mentions filter, loading older pages, and the unseen count with marking them seen.
//!
//! The app keeps a [`Notifications`], passes it every [`Msg`] with
//! [`Notifications::update`], and does what the returned [`Event`]s ask. Bluesky is
//! reached through [`NotificationsApi`](crate::api::service::NotificationsApi). Replies,
//! mentions and quotes show the post itself, which the app draws, so the list is drawn
//! through [`View`] with a [`Host`].

mod group;
mod message;
mod state;
mod update;
mod view;

pub use group::Filter;
pub use message::{Ctx, Event, Msg};
pub use state::Notifications;
pub use view::{Host, View};

type Out = crate::features::Update<Msg, Event>;

#[cfg(test)]
mod tests;
