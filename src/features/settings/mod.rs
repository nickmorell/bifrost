//! The Settings screen.
//!
//! Preferences live in `crate::storage::settings`, which the shell owns and saves, so
//! this feature never touches them directly: the shell hands it a [`Prefs`] snapshot to
//! draw and applies the [`Event`]s that come back. What the screen keeps for itself is
//! the "remove everything" confirmation.

mod message;
mod props;
mod state;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use message::{Event, Msg};
pub use props::{Prefs, Service, UpdateCheck};
pub use state::Screen;
pub use view::{Host, View};
