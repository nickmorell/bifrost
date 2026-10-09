//! Typing into the form and submitting it.

use super::{Event, Login, Msg};
use crate::api::{Credentials, DEFAULT_SERVICE};

impl Login {
    /// Returns what to do when the form is submitted.
    pub fn update(&mut self, msg: Msg) -> Option<Event> {
        match msg {
            Msg::Identifier(v) => self.identifier = v.trim().trim_start_matches('@').to_string(),
            Msg::Password(v) => self.password = v,
            Msg::Code(v) => self.code = Some(v.trim().to_string()),
            Msg::Service(v) => self.service = v.trim().to_string(),
            Msg::ToggleAdvanced => self.advanced = !self.advanced,
            Msg::Browser => {
                if self.busy {
                    return None;
                }
                self.busy = true;
                self.browser = true;
                self.error = None;
                return Some(Event::Browser(if self.identifier.is_empty() {
                    self.service_or_default()
                } else {
                    self.identifier.clone()
                }));
            }
            Msg::CancelBrowser => {
                self.busy = false;
                self.browser = false;
                return Some(Event::CancelBrowser);
            }
            Msg::Submit => {
                if self.busy || self.identifier.is_empty() || self.password.is_empty() {
                    return None;
                }
                if let Some(why) = crate::api::insecure_server(&self.service) {
                    self.error = Some(why.into());
                    return None;
                }
                self.busy = true;
                self.error = None;
                return Some(Event::Password(Credentials {
                    identifier: self.identifier.clone(),
                    password: self.password.clone(),
                    auth_factor_token: self.code.clone().filter(|c| !c.is_empty()),
                    service: self.service_or_default(),
                }));
            }
        }
        None
    }

    fn service_or_default(&self) -> String {
        if self.service.is_empty() {
            DEFAULT_SERVICE.into()
        } else {
            self.service.clone()
        }
    }
}
