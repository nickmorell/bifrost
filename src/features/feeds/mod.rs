//! Your feeds: the pinned ones as tabs on Home, and the Feeds screen with pinned,
//! saved and suggested feeds.
//!
//! Which feeds are saved and pinned is part of your Bluesky preferences, which the
//! app keeps, so the feature is handed them and asks for changes as events. The app
//! keeps a [`Feeds`], passes it every [`Msg`] with [`Feeds::update`], and does what
//! the returned [`Event`]s ask. Bluesky is reached through
//! [`FeedsApi`](crate::api::service::FeedsApi).

mod load;
mod message;
mod state;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use message::{Event, Msg};
pub use state::{Feeds, Tab};
pub use view::{Screen, Tabs, discover};

type Out = crate::features::Update<Msg, Event>;
