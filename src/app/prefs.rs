//! Your Bluesky preferences: loading them, and changing them here at once while the
//! changes are saved one at a time.

use super::{App, Message, TimelineKey, feeds};
use crate::api::ApiError;
use crate::features::prefs;
use crate::model::{FeedSource, PrefEdit, Preferences};
use iced::Task;

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(Result<Preferences, ApiError>),
    /// Changing and saving them (`features::prefs`).
    Save(prefs::Msg),
}

impl App {
    pub(super) fn prefs_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Loaded(Ok(prefs)) => {
                self.prefs = prefs;
                // Following may have loaded first; apply its filters now.
                let (prefs, me) = (&self.prefs.following, &self.my_did);
                self.timelines
                    .filter(&TimelineKey::Feed(FeedSource::Following), |i| {
                        prefs.keep(me, i)
                    });
                let names = self.feeds_update(feeds::Msg::Reset);
                // Re-check the home timeline now that moderation preferences are known.
                Task::batch([names, self.load_current(false)])
            }
            Msg::Save(msg) => {
                let out = self.pref_saver.update(msg, &mut self.prefs, &self.client);
                let mut tasks = vec![out.task.map(|m| Message::Prefs(Msg::Save(m)))];
                for event in out.events {
                    tasks.push(match event {
                        prefs::Event::Changed => self.feeds_update(feeds::Msg::Sync),
                        prefs::Event::Failed(e) => self.action_failed("save that setting", e),
                    });
                }
                Task::batch(tasks)
            }
            Msg::Loaded(Err(e)) => {
                self.feeds.following_only();
                self.toast = Some(format!("Could not load your preferences: {e}"));
                self.load_current(false)
            }
        }
    }
}

/// The message that changes a preference here at once and saves it.
pub(super) fn edit(edit: PrefEdit) -> Message {
    Message::Prefs(Msg::Save(prefs::Msg::Edit(edit)))
}
