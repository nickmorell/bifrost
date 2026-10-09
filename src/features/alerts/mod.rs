//! OS notifications: while BiFrost is in the background it checks for new
//! notifications and direct messages and turns the kinds you chose into [`Notice`]s
//! for the app to show. Anything that arrived while the window had focus was seen
//! there, so it never pops up later.

mod message;
mod notice;
mod state;
mod update;

pub use message::{Ctx, Msg, Notice};
pub use state::Alerts;

#[cfg(test)]
mod tests;
