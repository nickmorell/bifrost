//! Typing muted words, and writing and sending reports.

use super::state::Report;
use super::{Event, Moderation, Msg, Out};
use crate::api::service::ModerationApi;
use crate::features::Update;
use crate::model::PrefEdit;
use iced::Task;

impl Moderation {
    pub fn update(&mut self, msg: Msg, api: &impl ModerationApi) -> Out {
        match msg {
            Msg::Edit(edit) => Update::event(Event::Edit(edit)),
            Msg::WordInput(word) => {
                self.muted_word = word.chars().take(100).collect();
                Update::none()
            }
            Msg::AddWord => {
                let word = std::mem::take(&mut self.muted_word);
                if word.trim().trim_start_matches('#').is_empty() {
                    return Update::none();
                }
                Update::event(Event::Edit(PrefEdit::MuteWord(word)))
            }
            Msg::People(kind) => Update::event(Event::People(kind)),
            Msg::Report(subject) => {
                self.report = Some(Report {
                    subject,
                    reason: None,
                    details: String::new(),
                    sending: false,
                });
                Update::none()
            }
            Msg::Reason(reason) => {
                if let Some(r) = &mut self.report {
                    r.reason = Some(reason);
                }
                Update::none()
            }
            Msg::Details(details) => {
                if let Some(r) = &mut self.report {
                    r.details = details.chars().take(2000).collect();
                }
                Update::none()
            }
            Msg::Submit => {
                let Some(r) = &mut self.report else {
                    return Update::none();
                };
                let Some(reason) = r.reason else {
                    return Update::none();
                };
                r.sending = true;
                let (api, subject, details) = (api.clone(), r.subject.clone(), r.details.clone());
                Update::task(Task::perform(
                    async move { api.report(&subject, reason, &details).await },
                    Msg::Sent,
                ))
            }
            Msg::Cancel => {
                self.report = None;
                Update::none()
            }
            Msg::Sent(result) => {
                self.report = None;
                Update::event(match result {
                    Ok(()) => Event::Toast("Report sent to Bluesky moderation. Thank you.".into()),
                    Err(e) => Event::Failed(e),
                })
            }
        }
    }

    /// Forgets the word being typed and any report being written.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
