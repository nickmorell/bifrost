//! The app's side of the composer (`features::composer`): opening it, passing it
//! messages, doing what its events ask and floating its dialog over the screen.

use super::{App, Message, post, prefs};
use crate::features::Update;

pub use crate::features::composer::{Composer, Msg, Saved};
use crate::features::composer::{Event, Host, View};
use crate::media::files::Local;
use crate::model::{Interaction, PostRecord, PrefEdit};
use crate::storage::drafts::{Draft, NewDraft};
use crate::theme::size;
use iced::{Element, Task};

impl App {
    /// A new composer with your language and who-can-reply settings.
    pub(super) fn new_composer(&self, c: Composer) -> Composer {
        c.with_lang(self.post_lang())
            .with_interaction(self.prefs.interaction.clone())
            .with_alt_reminder(self.settings.alt_reminder)
    }

    pub(super) fn composer_update(&mut self, msg: Msg) -> Task<Message> {
        let Some(c) = &mut self.composer else {
            return Task::none();
        };
        let out = c.update(msg, &self.client, &Local);
        self.composer_done(out)
    }

    /// Runs the composer's task and does what its events ask, in order.
    fn composer_done(&mut self, out: Update<Msg, Event>) -> Task<Message> {
        let mut tasks = vec![out.task.map(Message::Composer)];
        for event in out.events {
            tasks.push(match event {
                Event::Close => {
                    self.composer = None;
                    Task::none()
                }
                Event::SaveDraft => {
                    self.stash_composer();
                    Task::none()
                }
                Event::Tags(tags) => {
                    self.settings.remember_tags(tags);
                    self.save_settings();
                    Task::none()
                }
                Event::Posted(reply_parent) => self.posted(reply_parent),
                Event::Images(urls) => self.request_images(urls),
                Event::NeedMyLists => self.load_my_lists(),
                Event::KeepInteraction(i) => self.handle(prefs::edit(PrefEdit::PostInteraction(i))),
                Event::Schedule(at) => {
                    self.schedule_composer(&at.to_rfc3339());
                    Task::none()
                }
            });
        }
        Task::batch(tasks)
    }

    pub(super) fn composer_view<'a>(&'a self, c: &'a Composer) -> Element<'a, Message> {
        View {
            composer: c,
            host: self,
            lift: Message::Composer,
        }
        .dialog()
    }
}

/// The post as it is kept in Drafts.
pub(super) fn to_draft(saved: Saved) -> NewDraft {
    NewDraft {
        text: saved.text,
        reply_to: saved.reply_to,
        quote: saved.quote,
        lang: saved.lang,
        images: saved.images,
        send_at: None,
    }
}

/// A draft with its images, to reopen in the composer.
pub(super) fn from_draft(draft: &Draft, images: Vec<(Vec<u8>, String, u32, u32)>) -> Saved {
    Saved {
        text: draft.text.clone(),
        reply_to: draft.reply_to.clone(),
        quote: draft.quote.clone(),
        lang: draft.lang.clone(),
        images,
    }
}

impl Host<Message> for App {
    fn rich_text<'a>(&'a self, record: &'a PostRecord) -> Element<'a, Message> {
        post::post_text(self.look(), record, size::BODY)
    }

    fn my_avatar(&self) -> Option<&str> {
        self.me.as_ref().and_then(|m| m.avatar.as_deref())
    }

    fn tag_suggestions(&self, prefix: &str) -> Vec<String> {
        App::tag_suggestions(self, prefix)
    }

    fn default_interaction(&self) -> &Interaction {
        &self.prefs.interaction
    }

    fn interaction_panel<'a>(
        &'a self,
        current: &Interaction,
        threads: bool,
        on_change: &dyn Fn(Interaction) -> Message,
    ) -> Element<'a, Message> {
        super::interaction::panel(self, current, threads, on_change)
    }
}
