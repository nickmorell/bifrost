//! Lists of accounts: someone's followers and follows, and your muted and blocked
//! accounts, loaded a page at a time.
//!
//! The app keeps a [`People`], passes it every [`Msg`] with [`People::update`], and
//! does what the returned [`Event`]s ask. Bluesky is reached through
//! [`PeopleApi`](crate::api::service::PeopleApi). [`person`] draws one account, for
//! other screens that list people.

mod message;
mod state;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use message::{Event, Msg};
pub use state::People;
pub use view::{View, person};

type Out = crate::features::Update<Msg, Event>;
