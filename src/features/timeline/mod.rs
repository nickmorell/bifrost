//! Timelines: pages of posts from a feed or list, someone's profile, or your
//! bookmarks. Loading the first page, the next ones and the newest on top, checking
//! for new posts in the background, and drawing a long timeline cheaply.
//!
//! Timelines are kept for every screen and deck column that shows one, so the app
//! holds one [`Timelines`] and asks it for the one it is showing. Posts are drawn by
//! the app through [`Host`].

mod filter;
mod key;
mod merge;
mod message;
mod place;
mod state;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use filter::filter_menu;
pub use key::{Key, Load};
pub use merge::item_key;
pub use message::{Ctx, Event, Msg};
pub use state::{Removed, Timeline, Timelines};
pub use view::{Host, Shape, View};
