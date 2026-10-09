//! Search: the search box with suggestions while typing, results for posts, people
//! and feeds, `#tag` pages, and the Explore page shown before anything is searched.
//!
//! Posts, people and feed rows are drawn by the app through [`Host`]. Recent searches
//! live in the app's settings, so the views get them as data and changes to them are
//! [`Event`]s.

mod message;
mod state;
mod tab;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use message::{Event, Msg};
pub use state::{INPUT, Search};
pub use tab::Tab;
pub use view::{Host, View};
