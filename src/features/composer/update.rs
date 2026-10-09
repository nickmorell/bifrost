//! What each message does to the composer.

use super::Out;
use super::message::{Event, Msg};
use super::schedule;
use super::state::Composer;
use crate::api::richtext;
use crate::api::service::ComposerApi;
use crate::api::{DraftImage, PostDraft};
use crate::features::Update;
use crate::media::files::Attachments;
use crate::model::StrongRef;
use iced::Task;
use iced::widget::text_editor::{Action, Edit};
use std::sync::Arc;

impl Composer {
    /// Handles one message. `api` and `files` are cloned into the tasks that need them.
    pub fn update<A: ComposerApi, F: Attachments>(&mut self, msg: Msg, api: &A, files: &F) -> Out {
        match msg {
            Msg::Edit(action) => {
                let edited = action.is_edit();
                self.content.perform(action);
                if !edited {
                    // Moving the cursor away from a word ends its suggestions.
                    if self.mention_partial().is_none() {
                        self.suggestions.clear();
                    }
                    self.tag_partial = self.tag_at_cursor();
                    return Update::none();
                }
                Update::task(self.after_edit(api))
            }
            Msg::Suggestions(q, Ok(found)) => {
                if self.mention_partial().as_deref() == Some(q.as_str()) {
                    self.suggestions = found;
                }
                let avatars = self.suggestions.iter().filter_map(|a| a.avatar.clone());
                Update::event(Event::Images(avatars.collect()))
            }
            Msg::Suggestions(_, Err(_)) => Update::none(),
            Msg::PickMention(handle) => {
                self.replace_word(&format!("@{handle} "));
                self.suggestions.clear();
                Update::none()
            }
            Msg::PickTag(tag) => {
                self.replace_word(&format!("#{tag} "));
                self.tag_partial = None;
                Update::none()
            }
            Msg::ToggleEmoji => {
                self.emoji_open = !self.emoji_open;
                self.warnings_open = false;
                Update::none()
            }
            Msg::Emoji(e) => {
                self.content
                    .perform(Action::Edit(Edit::Paste(Arc::new(e.to_string()))));
                Update::task(self.after_edit(api))
            }
            Msg::ToggleWarnings => {
                self.warnings_open = !self.warnings_open;
                self.emoji_open = false;
                Update::none()
            }
            Msg::ToggleLabel(label) => {
                if let Some(i) = self.labels.iter().position(|l| *l == label) {
                    self.labels.remove(i);
                } else {
                    self.labels.push(label);
                }
                Update::none()
            }
            Msg::CloseMenus => {
                self.schedule_open = false;
                self.emoji_open = false;
                self.warnings_open = false;
                self.interaction_open = false;
                Update::none()
            }
            Msg::ToggleInteraction => {
                self.interaction_open = !self.interaction_open;
                self.emoji_open = false;
                self.warnings_open = false;
                Update::event(Event::NeedMyLists)
            }
            Msg::Interaction(i) => {
                self.interaction = i;
                Update::none()
            }
            Msg::KeepInteraction(i) => Update::event(Event::KeepInteraction(i)),
            Msg::PasteText(Some(t)) => {
                self.content.perform(Action::Edit(Edit::Paste(Arc::new(t))));
                Update::task(self.after_edit(api))
            }
            Msg::PasteText(None) => Update::none(),
            Msg::CardLoaded(url, result) => {
                if self.card_url.as_deref() == Some(url.as_str())
                    && let Ok(card) = result
                {
                    let thumb = card.image_url.clone();
                    self.card = Some(card);
                    return Update::event(Event::Images(thumb.into_iter().collect()));
                }
                Update::none()
            }
            Msg::RemoveCard => {
                self.card_dismissed = self.card_url.take();
                self.card = None;
                Update::none()
            }
            Msg::Lang(l) => {
                self.lang = l.trim().to_lowercase().chars().take(8).collect();
                Update::none()
            }
            Msg::Cancel if self.is_empty() => Update::event(Event::Close),
            Msg::Cancel => {
                self.confirm_close = true;
                Update::none()
            }
            Msg::SaveDraft => Update::event(Event::SaveDraft),
            Msg::Discard => Update::event(Event::Close),
            Msg::KeepEditing => {
                self.confirm_close = false;
                Update::none()
            }
            Msg::ToggleSchedule => {
                self.schedule_open = !self.schedule_open;
                self.emoji_open = false;
                self.warnings_open = false;
                self.interaction_open = false;
                Update::none()
            }
            Msg::ToggleThread => {
                let on = !self.threaded();
                self.thread = on;
                self.no_split = !on;
                Update::none()
            }
            Msg::ImagePart(i) => {
                let parts = self.parts().len().max(1);
                if let Some(a) = self.images.get_mut(i) {
                    a.part = (a.part.min(parts - 1) + 1) % parts;
                }
                Update::none()
            }
            Msg::ScheduleText(t) => {
                self.schedule_text = t;
                Update::none()
            }
            Msg::ScheduleTyped => {
                if let Some(at) = schedule::typed(&self.schedule_text, chrono::Utc::now()) {
                    return self.update(Msg::Schedule(at), api, files);
                }
                self.error = Some("Type a time that is still ahead, like 2026-10-09 14:30.".into());
                Update::none()
            }
            Msg::Schedule(at) => {
                self.schedule_open = false;
                if self.video.is_some() || self.threaded() || !self.can_post(self.text().trim_end())
                {
                    return Update::none();
                }
                Update::event(Event::Schedule(at))
            }
            Msg::Submit => {
                if self.alt_reminder && self.missing_alt() > 0 && !self.posting {
                    self.confirm_alt = true;
                    return Update::none();
                }
                self.submit(api)
            }
            Msg::PostAnyway => {
                self.confirm_alt = false;
                self.submit(api)
            }
            Msg::AddAlt => {
                self.confirm_alt = false;
                Update::none()
            }
            Msg::Posted(Ok(_)) => Update::event(Event::Posted(self.reply_parent_uri())),
            Msg::Posted(Err(e)) => {
                self.failed(&e.to_string());
                Update::none()
            }
            other => self.attach_update(other, files),
        }
    }

    /// Suggestions and the link card after the text changed.
    pub(super) fn after_edit<A: ComposerApi>(&mut self, api: &A) -> Task<Msg> {
        self.error = None;
        let text = self.text();
        let mut tasks = Vec::new();

        match self.mention_partial() {
            Some(partial) if partial.len() >= 2 => {
                let api = api.clone();
                tasks.push(Task::perform(
                    async move {
                        let r = api.typeahead(&partial).await;
                        (partial, r)
                    },
                    |(q, r)| Msg::Suggestions(q, r),
                ));
            }
            _ => self.suggestions.clear(),
        }
        self.tag_partial = self.tag_at_cursor();

        // Link card for the first link, unless images are attached.
        let link = richtext::first_link(&text);
        if link != self.card_url && link != self.card_dismissed {
            self.card = None;
            self.card_url.clone_from(&link);
            if let Some(url) = link {
                let api = api.clone();
                tasks.push(Task::perform(
                    async move {
                        let r = api.link_card(&url).await;
                        (url, r)
                    },
                    |(u, r)| Msg::CardLoaded(u, r),
                ));
            }
        }
        Task::batch(tasks)
    }

    /// Sends the post, unless it can't be sent as it is, and offers its hashtags for
    /// suggestions next time.
    fn submit<A: ComposerApi>(&mut self, api: &A) -> Out {
        let draft = self.draft();
        if self.posting || !self.can_post(&draft.text) {
            return Update::none();
        }
        self.posting = true;
        self.error = None;
        let tags: Vec<String> = richtext::detect(&draft.text)
            .into_iter()
            .filter_map(|d| match d.kind {
                richtext::DetectedKind::Tag(t) => Some(t),
                _ => None,
            })
            .collect();
        let api = api.clone();
        let thread = if self.threaded() {
            self.thread_drafts()
        } else {
            Vec::new()
        };
        let out = Update::task(Task::perform(
            async move {
                if thread.len() > 1 {
                    api.create_thread(thread).await
                } else {
                    api.create_post(thread.into_iter().next().unwrap_or(draft))
                        .await
                }
            },
            Msg::Posted,
        ));
        if tags.is_empty() {
            out
        } else {
            out.and(Event::Tags(tags))
        }
    }

    /// The post as it would be sent now.
    pub(super) fn draft(&self) -> PostDraft {
        let text = self.text().trim_end().to_string();
        let reply = self.reply_to.as_ref().map(|p| {
            let parent = StrongRef {
                uri: p.uri.clone(),
                cid: p.cid.clone(),
            };
            let root = p
                .record
                .reply
                .as_ref()
                .map_or_else(|| parent.clone(), |r| r.root.clone());
            (root, parent)
        });
        let media = self.has_media();
        PostDraft {
            text,
            reply,
            quote: self.quote.as_ref().map(|q| StrongRef {
                uri: q.uri.clone(),
                cid: q.cid.clone(),
            }),
            images: self
                .images
                .iter()
                .map(|a| DraftImage {
                    bytes: a.image.bytes.clone(),
                    alt: a.alt.clone(),
                    width: a.image.width,
                    height: a.image.height,
                })
                .collect(),
            video: self.video.clone(),
            link_card: if media { None } else { self.card.clone() },
            labels: if media {
                self.labels.iter().map(ToString::to_string).collect()
            } else {
                Vec::new()
            },
            langs: if self.lang.is_empty() {
                vec![]
            } else {
                vec![self.lang.clone()]
            },
            interaction: self.interaction.clone(),
        }
    }
}
