//! The login form as typed so far.

use crate::api::{ApiError, DEFAULT_SERVICE};

#[derive(Debug)]
pub struct Login {
    pub(super) identifier: String,
    pub(super) password: String,
    pub(super) code: Option<String>,
    pub(super) service: String,
    pub(super) advanced: bool,
    pub(super) busy: bool,
    /// Waiting for the person to finish signing in in their browser.
    pub(super) browser: bool,
    pub(super) error: Option<String>,
    pub(super) notice: Option<&'static str>,
}

impl Default for Login {
    fn default() -> Self {
        Self {
            identifier: String::new(),
            password: String::new(),
            code: None,
            service: DEFAULT_SERVICE.to_string(),
            advanced: false,
            busy: false,
            browser: false,
            error: None,
            notice: None,
        }
    }
}

impl Login {
    pub fn expired() -> Self {
        Self {
            notice: Some("Your session expired. Please log in again."),
            ..Self::default()
        }
    }

    pub fn adding_account() -> Self {
        Self {
            notice: Some("Log in to another account. You can switch between accounts in Settings."),
            ..Self::default()
        }
    }

    /// Signing in did not work, or needs the code Bluesky emailed.
    pub fn failed(&mut self, error: &ApiError) {
        self.busy = false;
        self.browser = false;
        if *error == ApiError::NeedsCode {
            self.code = Some(String::new());
            self.error = Some("Bluesky sent a sign-in code to your email. Enter it below.".into());
        } else {
            self.error = Some(error.to_string());
        }
    }
}
