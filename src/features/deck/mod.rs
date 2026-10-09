//! Deck mode: Home as several timelines side by side, each in its own column.
//!
//! The columns are kept with the app's settings, so the deck is handed them and
//! reports changes to save as events. What a column shows (a timeline or the
//! notifications) is drawn by the app through [`Host`].

mod message;
mod state;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use message::{Event, Msg};
pub use state::{Deck, MAX_COLUMNS, scroll_id, starting};
pub use view::{AddColumn, Host, View};
