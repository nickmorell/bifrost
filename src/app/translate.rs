//! The app's side of translating posts (`features::translate`): passing it messages,
//! saving the choices it reports and placing its views.

use super::{App, Message, nav};
use crate::features::translate::{self, Ctx, Event};
pub use crate::features::translate::{Msg, Translations};
use crate::model::PostView;
use crate::translate::Translator;
use iced::{Element, Task};

/// The message that translates a post.
pub fn translate_post(post: &PostView) -> Message {
    Message::Translate(translate::translate(post))
}

impl App {
    /// The language posts are translated into.
    pub(super) fn translate_to(&self) -> String {
        self.settings
            .translate_to
            .clone()
            .unwrap_or_else(|| self.post_lang())
    }

    fn translate_ctx(&self) -> Ctx {
        Ctx {
            service: self.settings.translate_service,
            to: self.translate_to(),
            server: self.settings.translate_server.clone(),
        }
    }

    pub(super) fn translate_update(&mut self, msg: Msg) -> Task<Message> {
        let ctx = self.translate_ctx();
        let out = self.translations.update(msg, &ctx, &Translator::default());
        let mut tasks = vec![out.task.map(Message::Translate)];
        for event in out.events {
            match event {
                Event::OpenUrl(url) => {
                    tasks.push(self.handle(Message::Nav(nav::Msg::OpenUrl(url))));
                }
                Event::To(code) => {
                    self.settings.translate_to = Some(code);
                    self.save_settings();
                }
                Event::Service(s) => {
                    self.settings.translate_service = s;
                    self.save_settings();
                }
                Event::Server(url) => {
                    self.settings.translate_server = url;
                    self.save_settings();
                }
                Event::Toast(t) => self.toast = Some(t),
            }
        }
        Task::batch(tasks)
    }

    /// Under a post's text: its translation, or a quiet "Translate" link when it is in
    /// another language. Nothing otherwise.
    pub(super) fn translation_view<'a>(
        &'a self,
        post: &'a PostView,
        text_size: f32,
    ) -> Option<Element<'a, Message>> {
        let ctx = self.translate_ctx();
        translate::under_post(&self.translations, post, text_size, &ctx)
            .map(|e| e.map(Message::Translate))
    }

    /// The Translation card in Settings.
    pub(super) fn translate_settings(&self) -> Element<'_, Message> {
        translate::settings(&self.translations, &self.translate_ctx()).map(Message::Translate)
    }
}
