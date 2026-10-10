//! The opt-in check for a newer BiFrost. It runs once per start, when "Check for
//! updates" is on in Settings, and only says what it found: BiFrost never updates itself.

use super::{App, Message};
use crate::api::ApiError;
use crate::api::github::{self, Release};
use crate::features::settings::UpdateCheck;
use iced::Task;

#[derive(Debug, Clone)]
pub enum Msg {
    /// Look for a newer version, unless this run already knows the answer.
    Check,
    Checked(Result<Option<Release>, ApiError>),
}

impl App {
    pub(super) fn updates_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Check => {
                if !self.settings.check_updates
                    || matches!(
                        self.update,
                        UpdateCheck::UpToDate | UpdateCheck::Available { .. }
                    )
                {
                    return Task::none();
                }
                self.update = UpdateCheck::Checking;
                Task::perform(github::newer_release(), |r| {
                    Message::Updates(Msg::Checked(r))
                })
            }
            Msg::Checked(result) => {
                if self.settings.check_updates {
                    self.update = match result {
                        Ok(Some(r)) => UpdateCheck::Available {
                            version: r.version,
                            url: r.url,
                        },
                        Ok(None) => UpdateCheck::UpToDate,
                        Err(e) => UpdateCheck::Failed(e.friendly()),
                    };
                }
                Task::none()
            }
        }
    }

    /// Switches the check on (and checks) or off (and forgets what it found).
    pub(super) fn set_check_updates(&mut self, on: bool) -> Task<Message> {
        self.settings.check_updates = on;
        self.save_settings();
        if on {
            self.updates_update(Msg::Check)
        } else {
            self.update = UpdateCheck::Off;
            Task::none()
        }
    }

    /// What Settings shows for the check.
    pub(super) fn update_check(&self) -> UpdateCheck {
        match (&self.update, self.settings.check_updates) {
            (_, false) => UpdateCheck::Off,
            (UpdateCheck::Off, true) => UpdateCheck::Checking,
            (found, true) => found.clone(),
        }
    }

    /// Opens a new GitHub issue in the browser with this build's details filled in.
    pub(super) fn report_problem(&mut self) -> Task<Message> {
        let url = github::new_issue_url(std::env::consts::OS, std::env::consts::ARCH);
        self.handle(Message::Nav(super::nav::Msg::Browse(url)))
    }
}
