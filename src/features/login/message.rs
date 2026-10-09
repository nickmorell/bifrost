//! What the login form is told, and what it asks of the app.

use crate::api::Credentials;

#[derive(Debug, Clone)]
pub enum Msg {
    Identifier(String),
    Password(String),
    Code(String),
    Service(String),
    ToggleAdvanced,
    Submit,
    /// Sign in through the browser instead of with an app password.
    Browser,
    CancelBrowser,
}

/// What the app has to do for the form.
#[derive(Debug)]
pub enum Event {
    /// Sign in with an app password.
    Password(Credentials),
    /// Sign in through the browser: a handle, or the hosting provider's URL.
    Browser(String),
    /// Stop waiting for the browser sign-in.
    CancelBrowser,
}
