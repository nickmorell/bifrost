//! The app's side of profiles (`features::profile`): passing them messages, doing
//! what their events ask, and drawing the profile screen and the "Edit profile"
//! dialog with the posts, lists and starter packs the app draws.

use super::{App, Message, Route, Toggle, actions, chat, lists, nav};
use crate::api::ReportSubject;
use crate::features::profile::{Ctx, Event, Host, Msg, Tab, View};
use crate::media::files::Local;
use crate::model::ListInfo;
use crate::moderation::Context;
use iced::{Element, Task};

fn toggle(on: bool) -> Toggle {
    if on { Toggle::On } else { Toggle::Off }
}

impl App {
    /// Handles one profile message and does what it asks of the app, in order.
    pub(super) fn profile_update(&mut self, msg: Msg) -> Task<Message> {
        let ctx = Ctx {
            me: self.me.as_ref(),
        };
        let out = self.profile.update(msg, &ctx, &self.client, &Local);
        let mut tasks = vec![out.task.map(Message::Profile)];
        for event in out.events {
            tasks.push(match event {
                Event::Pictures(urls) => self.request_images(urls),
                Event::Failed(e) => self.handle_session_error(e),
                Event::Tab(actor, tab) => {
                    if let Some(r @ Route::Profile(..)) = self.routes.last_mut() {
                        *r = Route::Profile(actor, tab);
                    }
                    Task::batch([self.on_enter(), self.restore_scroll()])
                }
                Event::Follow(did, on) => {
                    self.handle(Message::Action(actions::Msg::Follow(did, toggle(on))))
                }
                Event::Mute(did, on) => {
                    self.handle(Message::Action(actions::Msg::Mute(did, toggle(on))))
                }
                Event::Block(did, on) => {
                    self.handle(Message::Action(actions::Msg::Block(did, toggle(on))))
                }
                Event::Message(did) => self.handle(Message::Chat(chat::Msg::MessagePerson(did))),
                Event::AddToLists(did, name) => {
                    self.handle(Message::Lists(lists::Msg::AddTo(did, name)))
                }
                Event::Report(did) => self.handle(Message::Moderation(
                    super::moderation::Msg::Report(ReportSubject::Account(did)),
                )),
                Event::People(actor, kind) => {
                    self.handle(Message::Nav(nav::Msg::OpenPeople(actor, kind)))
                }
                Event::OpenFeed(source, name) => {
                    self.handle(Message::Nav(nav::Msg::OpenFeed(source, name)))
                }
                Event::OpenUrl(url) => self.handle(Message::Nav(nav::Msg::OpenUrl(url))),
                Event::Saved => {
                    self.toast = Some("Profile saved.".into());
                    self.reload_me()
                }
            });
        }
        Task::batch(tasks)
    }

    /// Loads what the profile of `actor` shows on `tab`.
    pub(super) fn open_profile(&mut self, actor: String, tab: Tab) -> Task<Message> {
        let packs = if tab == Tab::StarterPacks {
            self.load_actor_packs(actor.clone())
        } else {
            Task::none()
        };
        let profile = self.profile_update(Msg::Open(actor, tab));
        Task::batch([profile, packs, self.load_current(false)])
    }

    /// Loads your lists, if not loaded yet, for choosing which can reply.
    pub(super) fn load_my_lists(&mut self) -> Task<Message> {
        self.profile_update(Msg::LoadLists(self.my_did.to_string()))
    }

    /// The profile screen.
    pub(super) fn profile_view<'a>(&'a self, actor: &'a str, tab: Tab) -> Element<'a, Message> {
        let body = View {
            profiles: &self.profile,
            actor,
            tab,
            me: &self.my_did,
            can_edit: self.me.is_some(),
            chat_opening: self.chats.opening,
            host: self,
            lift: Message::Profile,
        }
        .body();
        self.scroll(body)
    }

    /// The "Edit profile" dialog, while it is open.
    pub(super) fn profile_editor(&self) -> Option<Element<'_, Message>> {
        self.profile
            .editor()
            .map(|e| e.view(self, |m| Message::Profile(Msg::Editor(m))))
    }
}

impl Host<Message> for App {
    fn posts<'a>(&'a self, actor: &str, tab: Tab) -> Element<'a, Message> {
        let key = tab
            .filter()
            .map(|f| super::TimelineKey::Author(actor.to_string(), f));
        self.timeline_view(key.as_ref(), Context::Profile, &self.scroll_key())
    }

    fn starter_packs<'a>(&'a self, actor: &str, empty: &'static str) -> Element<'a, Message> {
        self.pack_cards(actor, empty)
    }

    fn list_card<'a>(&'a self, list: &'a ListInfo) -> Element<'a, Message> {
        lists::list_card(self, list)
    }

    fn linked_text<'a>(&'a self, text: &'a str) -> Element<'a, Message> {
        super::post::linked_text(self.look(), text, crate::theme::size::BODY)
    }

    fn retry<'a>(&self, why: String) -> Element<'a, Message> {
        super::util::retry(why)
    }
}
