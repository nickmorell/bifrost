//! Profiles: someone's header, posts, feeds, lists and starter packs, and the "Edit
//! profile" dialog for your own.
//!
//! The posts on a profile are a timeline and its starter packs belong to lists, so the
//! app draws those through [`Host`]; this feature keeps the profiles and the feeds and
//! lists people made, which other screens read too.

mod edit;
mod message;
mod state;
mod tab;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use edit::{Editor, Msg as EditMsg};
pub use message::{Ctx, Event, Msg};
pub use state::Profiles;
pub use tab::Tab;
pub use view::{Host, View};
