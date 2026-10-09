//! Translating, choosing a service and saving its key.

use super::{Ctx, Event, KEYED, Msg, State, Translations};
use crate::api::service::TranslateApi;
use crate::features::Update;
use crate::model::{TranslateService as Service, browser_url};
use iced::Task;

type Out = Update<Msg, Event>;

impl Translations {
    pub fn update(&mut self, msg: Msg, ctx: &Ctx, api: &impl TranslateApi) -> Out {
        match msg {
            Msg::Translate { uri, text, lang } => {
                if ctx.service == Service::Browser {
                    return Update::event(Event::OpenUrl(browser_url(&text, &ctx.to)));
                }
                if matches!(self.posts.get(&uri), Some(State::Loading)) {
                    return Update::none();
                }
                self.posts.insert(uri.clone(), State::Loading);
                let (api, ctx) = (api.clone(), ctx.clone());
                Update::task(Task::perform(
                    async move {
                        api.translate(ctx.service, text, lang, ctx.to, ctx.server)
                            .await
                    },
                    move |r| Msg::Done(uri.clone(), r),
                ))
            }
            Msg::Done(uri, result) => {
                // Hidden while it loaded: leave it hidden.
                if let Some(state) = self.posts.get_mut(&uri) {
                    *state = match result {
                        Ok(t) => State::Done(t),
                        Err(e) => State::Failed(e.to_string()),
                    };
                }
                Update::none()
            }
            Msg::Hide(uri) => {
                self.posts.remove(&uri);
                Update::none()
            }
            Msg::To(code) => {
                // Old translations are in the old language.
                self.posts.clear();
                Update::event(Event::To(code))
            }
            Msg::Service(s) => {
                self.key_input.clear();
                self.posts.clear();
                Update::event(Event::Service(s))
            }
            Msg::Server(url) => Update::event(Event::Server(url)),
            Msg::KeyInput(k) => {
                self.key_input = k;
                Update::none()
            }
            Msg::SaveKey => {
                let key = std::mem::take(&mut self.key_input);
                let (api, service) = (api.clone(), ctx.service);
                let saved = !key.trim().is_empty();
                Update::task(Task::perform(
                    async move { api.set_key(service, key).await },
                    move |r| Msg::KeySaved(service, saved, r),
                ))
            }
            Msg::KeySaved(service, saved, Ok(())) => {
                self.has_key.insert(service, saved);
                self.posts.clear();
                let what = if saved { "saved" } else { "removed" };
                Update::event(Event::Toast(format!("{} key {what}.", service.name())))
            }
            Msg::KeySaved(_, _, Err(e)) => {
                Update::event(Event::Toast(format!("Couldn't save the key: {e}")))
            }
            Msg::ReadKeys => {
                let api = api.clone();
                Update::task(Task::perform(
                    async move {
                        let mut found = Vec::new();
                        for s in KEYED {
                            found.push((s, api.has_key(s).await));
                        }
                        found
                    },
                    Msg::KeysRead,
                ))
            }
            Msg::KeysRead(found) => {
                self.has_key.extend(found);
                Update::none()
            }
            Msg::ForgetKeys => {
                self.has_key.clear();
                let api = api.clone();
                Update::task(Task::perform(
                    async move {
                        let mut result = Ok(());
                        for s in KEYED {
                            if let Err(e) = api.set_key(s, String::new()).await {
                                result = Err(e);
                            }
                        }
                        result
                    },
                    Msg::KeysForgotten,
                ))
            }
            Msg::KeysForgotten(Ok(())) => Update::none(),
            Msg::KeysForgotten(Err(e)) => Update::event(Event::Toast(format!(
                "Couldn't remove a translation API key: {e}"
            ))),
        }
    }
}
