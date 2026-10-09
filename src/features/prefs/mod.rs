//! Changing your Bluesky preferences: each change shows here at once, and changes
//! are saved one at a time so none is lost. If a save is refused, the preferences
//! are read again from the server.

#[cfg(test)]
mod tests;

use crate::api::ApiError;
use crate::api::service::PrefsApi;
use crate::features::Update;
use crate::model::{PrefEdit, Preferences};
use iced::Task;

/// Changes waiting to be saved, and whether a save is running.
#[derive(Debug, Default)]
pub struct Saver {
    queue: Vec<PrefEdit>,
    saving: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Change a preference here at once and save it to the account.
    Edit(PrefEdit),
    Saved(Result<Preferences, ApiError>),
    /// The preferences re-read after a save failed.
    Reloaded(Result<Preferences, ApiError>),
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The preferences shown changed.
    Changed,
    /// A save was refused; the preferences are being read again.
    Failed(ApiError),
}

type Out = Update<Msg, Event>;

impl Saver {
    /// Forgets unsaved changes, as when switching accounts.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn update(&mut self, msg: Msg, prefs: &mut Preferences, api: &impl PrefsApi) -> Out {
        match msg {
            Msg::Edit(edit) => {
                let mut raw = std::mem::take(&mut prefs.raw);
                edit.apply(&mut raw);
                *prefs = Preferences::from_json(&raw);
                self.queue.push(edit);
                Update::event(Event::Changed).then(self.save(api))
            }
            Msg::Saved(Ok(saved)) => {
                self.saving = false;
                // Edits made while this save was running go on top of what the
                // server now has.
                let mut raw = saved.raw;
                for edit in &self.queue {
                    edit.apply(&mut raw);
                }
                *prefs = Preferences::from_json(&raw);
                Update::event(Event::Changed).then(self.save(api))
            }
            Msg::Saved(Err(e)) => {
                self.saving = false;
                self.queue.clear();
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.preferences().await },
                    Msg::Reloaded,
                ))
                .and(Event::Failed(e))
            }
            Msg::Reloaded(Ok(fresh)) if self.queue.is_empty() && !self.saving => {
                *prefs = fresh;
                Update::event(Event::Changed)
            }
            Msg::Reloaded(_) => Update::none(),
        }
    }

    /// Sends queued edits, unless a save is already running.
    fn save(&mut self, api: &impl PrefsApi) -> Out {
        if self.saving || self.queue.is_empty() {
            return Update::none();
        }
        self.saving = true;
        let (api, edits) = (api.clone(), std::mem::take(&mut self.queue));
        Update::task(Task::perform(
            async move { api.edit_preferences(edits).await },
            Msg::Saved,
        ))
    }
}
