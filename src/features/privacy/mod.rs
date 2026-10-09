//! The Privacy settings: keeping your posts out of Discover and away from people
//! who aren't signed in. Each switch is saved to Bluesky at once and put back if
//! Bluesky refuses. Bluesky has no private accounts, and the view says so.

mod view;

#[cfg(test)]
mod tests;

use crate::api::ApiError;
use crate::api::service::PrivacyApi;
use crate::features::Update;
use crate::model::Privacy as Wanted;
use iced::Task;

/// What you have asked of Bluesky, once read.
#[derive(Debug, Default)]
pub struct Privacy {
    pub(crate) now: Option<Result<Wanted, String>>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Load,
    Loaded(Result<Wanted, ApiError>),
    Discover(bool),
    LoggedOut(bool),
    /// A change was saved, or refused, in which case the switch goes back.
    Saved(Result<(), ApiError>),
}

#[derive(Debug)]
pub enum Event {
    /// A change could not be saved.
    Failed(ApiError),
}

impl Privacy {
    pub fn update(&mut self, msg: Msg, api: &impl PrivacyApi) -> Update<Msg, Event> {
        match msg {
            Msg::Load => {
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.privacy().await },
                    Msg::Loaded,
                ))
            }
            Msg::Loaded(r) => {
                self.now = Some(r.map_err(|e| e.friendly()));
                Update::none()
            }
            Msg::Discover(hide) => {
                let Some(Ok(now)) = &mut self.now else {
                    return Update::none();
                };
                now.hide_from_discover = hide;
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.set_hide_from_discover(hide).await },
                    Msg::Saved,
                ))
            }
            Msg::LoggedOut(hide) => {
                let Some(Ok(now)) = &mut self.now else {
                    return Update::none();
                };
                now.hide_logged_out = hide;
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.set_hide_logged_out(hide).await },
                    Msg::Saved,
                ))
            }
            Msg::Saved(Ok(())) => Update::none(),
            // Read again, so the switches show what Bluesky has.
            Msg::Saved(Err(e)) => self.update(Msg::Load, api).and(Event::Failed(e)),
        }
    }
}
