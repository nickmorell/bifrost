//! The login screen: sign in through the browser (OAuth), or with a handle and app
//! password plus the email code when the server asks.
//!
//! The app keeps a [`Login`] while nobody is signed in, passes it every [`Msg`] with
//! [`Login::update`], and does the signing in the returned [`Event`] asks for. When
//! that fails it hands the reason back with [`Login::failed`].

mod message;
mod state;
mod update;
mod view;

#[cfg(test)]
mod tests;

pub use message::{Event, Msg};
pub use state::Login;
