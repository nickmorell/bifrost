//! Lists and starter packs: the pages for one list or pack, your own lists and
//! packs, making and editing them, and putting people on your lists.
//!
//! The app keeps a [`Lists`], passes it every [`Msg`] with [`Lists::update`], and
//! does what the returned [`Event`]s ask. Bluesky is reached through
//! [`ListsApi`](crate::api::service::ListsApi).

mod edit;
mod effects;
mod message;
mod state;
mod update;
mod view;

pub use message::{Ctx, Event, Msg};
pub use state::{AddTo, Editing, Editor, ListScreen, ListTab, Lists};
pub use view::{ListParts, add_to, editor, list_card};

type Out = crate::features::Update<Msg, Event>;

#[cfg(test)]
mod tests;
