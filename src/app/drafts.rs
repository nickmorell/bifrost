//! Drafts: keeping what the composer had when it closes, the Drafts screen
//! (`features::drafts`), and opening a draft again.

use super::composer::{self, Composer};
use super::{App, Message};
use crate::features::drafts::{self, Event, Opening, Sending, View};

pub use crate::features::drafts::Msg;
use crate::storage::drafts::{Draft, DraftStore};
use iced::{Element, Task};

/// Drafts kept on this computer, and this account's, newest first.
#[derive(Default)]
pub(super) struct Drafts {
    pub(super) store: DraftStore,
    pub(super) list: Vec<Draft>,
    /// Scheduled drafts on their way out.
    sending: std::collections::HashSet<u64>,
}

impl App {
    pub(super) fn drafts_update(&mut self, msg: Msg) -> Task<Message> {
        let out = drafts::update(msg, &self.drafts.list, &self.client);
        let mut tasks = vec![out.task.map(Message::Drafts)];
        for event in out.events {
            match event {
                Event::Open(open) => {
                    let Opening {
                        draft,
                        reply_to,
                        quote,
                        reply_gone,
                    } = *open;
                    if reply_gone {
                        self.toast = Some(
                            "The post this draft replies to is gone, so it opens as a new post."
                                .into(),
                        );
                    }
                    let images = self.drafts.store.images(&draft);
                    self.stash_composer();
                    let mut saved = composer::from_draft(&draft, images);
                    if saved.lang.is_empty() {
                        saved.lang = self.post_lang();
                    }
                    self.composer = Some(
                        Composer::reopen(saved, reply_to, quote)
                            .with_interaction(self.prefs.interaction.clone())
                            .with_alt_reminder(self.settings.alt_reminder),
                    );
                    // Opening takes it out of Drafts; leaving the composer saves it again.
                    self.forget_draft(draft.id);
                }
                Event::Delete(id) => self.forget_draft(id),
                Event::Failed(e) => tasks.push(self.action_failed("open the draft", e)),
                Event::Due(ids) => {
                    for id in ids {
                        if let Some(task) = self.send_draft(id) {
                            tasks.push(task);
                        }
                    }
                }
                Event::Sent(id) => {
                    self.drafts.sending.remove(&id);
                    self.forget_draft(id);
                    self.toast = Some("Your scheduled post was sent.".into());
                }
                Event::SendFailed(id, e) => {
                    self.drafts.sending.remove(&id);
                    self.unschedule(id);
                    self.toast = Some(format!(
                        "Couldn't send your scheduled post: {}. It's kept in Drafts.",
                        e.friendly()
                    ));
                }
                Event::Unschedule(id) => self.unschedule(id),
            }
        }
        Task::batch(tasks)
    }

    /// Closes the composer, keeping anything written in Drafts.
    pub(super) fn stash_composer(&mut self) {
        let Some(c) = self.composer.take() else {
            return;
        };
        // A post being sent stays open until the server answers.
        if c.is_posting() {
            self.composer = Some(c);
            return;
        }
        if c.is_empty() || self.my_did.is_empty() {
            return;
        }
        match self
            .drafts
            .store
            .save(&self.my_did, composer::to_draft(c.saved()))
        {
            Ok(_) => {
                self.drafts.list = self.drafts.store.list(&self.my_did);
                self.toast = Some("Saved to Drafts.".into());
            }
            Err(e) => {
                // Never lose the post: keep the composer open and say why.
                self.composer = Some(c);
                self.toast = Some(format!("Couldn't save the draft: {e}"));
            }
        }
    }

    /// Keeps the post in Drafts to be sent at `send_at` (RFC 3339), and closes the
    /// composer.
    pub(super) fn schedule_composer(&mut self, send_at: &str) {
        let Some(c) = self.composer.take() else {
            return;
        };
        let mut draft = composer::to_draft(c.saved());
        draft.send_at = Some(send_at.to_string());
        match self.drafts.store.save(&self.my_did, draft) {
            Ok(_) => {
                self.drafts.list = self.drafts.store.list(&self.my_did);
                self.toast = Some(format!(
                    "Scheduled for {}. BiFrost sends it then if it's running.",
                    crate::components::full_time(send_at)
                ));
            }
            Err(e) => {
                self.composer = Some(c);
                self.toast = Some(format!("Couldn't schedule the post: {e}"));
            }
        }
    }

    /// Starts sending the scheduled draft `id`, unless it is already on its way.
    fn send_draft(&mut self, id: u64) -> Option<Task<Message>> {
        let draft = self.drafts.list.iter().find(|d| d.id == id)?.clone();
        if !self.drafts.sending.insert(id) {
            return None;
        }
        let sending = Sending {
            images: self.drafts.store.images(&draft),
            draft,
            interaction: self.prefs.interaction.clone(),
        };
        Some(self.drafts_update(Msg::Send(Box::new(sending))))
    }

    /// Whether any of this account's drafts waits to be sent.
    pub(super) fn has_scheduled(&self) -> bool {
        self.drafts.list.iter().any(|d| d.send_at.is_some())
    }

    fn unschedule(&mut self, id: u64) {
        if let Err(e) = self.drafts.store.schedule(id, None) {
            self.toast = Some(format!("Couldn't change the draft: {e}"));
        }
        self.drafts.list = self.drafts.store.list(&self.my_did);
    }

    fn forget_draft(&mut self, id: u64) {
        if let Err(e) = self.drafts.store.delete(id) {
            self.toast = Some(format!("Couldn't remove the draft: {e}"));
        }
        self.drafts.list = self.drafts.store.list(&self.my_did);
    }

    /// The Drafts screen.
    pub(super) fn drafts_view(&self) -> Element<'_, Message> {
        let list = View {
            drafts: &self.drafts.list,
        };
        self.scroll(list.view().map(Message::Drafts))
    }
}
