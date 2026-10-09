//! The post composer: text with mention and hashtag suggestions, emoji, images or a
//! video (picked, pasted or dropped) with alt text and content warnings, link card
//! preview, reply or quote context, language and who can interact.
//!
//! The app keeps an optional [`Composer`], passes it every [`Msg`] with
//! [`Composer::update`], and does what the returned [`Event`]s ask. Lookups go
//! through [`ComposerApi`](crate::api::service::ComposerApi), files and the clipboard
//! through [`Attachments`](crate::media::files::Attachments). The dialog is drawn
//! through [`View`], which needs a [`Host`] for the parts the app draws.

mod attach;
mod message;
mod schedule;
mod split;
mod state;
mod thread;
mod update;
mod view;

pub use message::{Event, Msg};
pub use state::{Composer, Saved};
pub use view::{Host, View};

type Out = crate::features::Update<Msg, Event>;

#[cfg(test)]
mod tests;
