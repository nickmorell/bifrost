//! The app's side of the lists feature (`features::lists`): passing it messages,
//! doing what its events ask, and placing its views on screens.

use super::{App, Message, Route, TimelineKey, account, nav};
use crate::features::Update;
use crate::features::lists::{self, AddTo, Ctx, Editor, Event, ListParts};

pub use crate::features::lists::{Lists, Msg};
use crate::model::{AtUri, FeedSource, ListInfo, Profile};
use crate::theme::gap;
use iced::widget::Column;
use iced::{Element, Task};

impl App {
    pub(super) fn lists_update(&mut self, msg: Msg) -> Task<Message> {
        let ctx = Ctx {
            me: &self.my_did,
            my_name: self.me.as_ref().map(Profile::name),
        };
        let out = self.lists.update(msg, &ctx, &self.client);
        self.lists_done(out)
    }

    /// Runs the feature's task and does what its events ask, in order.
    fn lists_done(&mut self, out: Update<Msg, Event>) -> Task<Message> {
        let mut tasks = vec![out.task.map(Message::Lists)];
        for event in out.events {
            tasks.push(match event {
                Event::OpenList(uri) => self.push_route(Route::List(uri)),
                Event::OpenPack(uri) => self.push_route(Route::StarterPack(uri)),
                Event::OpenProfile(did) => {
                    self.handle(Message::Nav(nav::Msg::OpenProfile(did.to_string())))
                }
                Event::OpenUrl(url) => self.handle(Message::Nav(nav::Msg::OpenUrl(url))),
                Event::OpenFeed(source, name) => {
                    self.handle(Message::Nav(nav::Msg::OpenFeed(source, name)))
                }
                Event::Retry => self.handle(Message::Nav(nav::Msg::Retry)),
                Event::SignInAgain => self.handle(Message::Account(account::Msg::Changed(Ok(())))),
                Event::Toast(t) => {
                    self.toast = Some(t);
                    Task::none()
                }
                Event::Error(e) => self.handle_error(e),
                Event::ErrorShown(e) => self.handle_session_error(e),
                Event::Images(urls) => self.request_images(urls),
                Event::TabChanged => self.load_current(false),
                Event::MyListsChanged => {
                    self.profile.forget_lists(&self.my_did);
                    Task::none()
                }
                Event::NeedMyLists => self.load_my_lists(),
                Event::Deleted(uri) => {
                    let open =
                        matches!(self.route(), Route::List(u) | Route::StarterPack(u) if *u == uri);
                    if open && self.routes.len() > 1 {
                        self.routes.pop();
                        self.on_enter()
                    } else {
                        self.load_lists_screen()
                    }
                }
            });
        }
        Task::batch(tasks)
    }

    /// Loads a list's details and first people, unless already loaded.
    pub(super) fn ensure_list(&mut self, uri: AtUri) -> Task<Message> {
        self.lists
            .ensure_list(&self.client, uri)
            .map(Message::Lists)
    }

    pub(super) fn load_pack(&mut self, uri: AtUri) -> Task<Message> {
        self.lists.load_pack(&self.client, uri).map(Message::Lists)
    }

    pub(super) fn load_actor_packs(&self, actor: String) -> Task<Message> {
        self.lists
            .load_actor_packs(&self.client, actor)
            .map(Message::Lists)
    }

    /// Your lists and starter packs, for the Lists screen.
    pub(super) fn load_lists_screen(&mut self) -> Task<Message> {
        let out = self.lists.load_lists_screen(&self.client, &self.my_did);
        self.lists_done(out)
    }

    // ---- Views ---------------------------------------------------------------------

    /// One list: its header, then its posts (a timeline) or its people.
    pub(super) fn list_view<'a>(&'a self, uri: &'a AtUri) -> Element<'a, Message> {
        let (top, people) = match self.lists.list_view(uri, self, &self.my_did) {
            ListParts::Status(e) => return e.map(Message::Lists),
            ListParts::Page { top, people } => (top, people),
        };
        let top = top.into_iter().map(|e| e.map(Message::Lists));
        let col = Column::with_children(top).spacing(gap::L);
        let body = if let Some(p) = people {
            p.map(Message::Lists)
        } else {
            let key = TimelineKey::Feed(FeedSource::List(uri.clone()));
            self.timeline_view(
                Some(&key),
                crate::moderation::Context::Feed,
                &self.scroll_key(),
            )
        };
        self.scroll(col.push(body))
    }

    /// A starter pack: who made it, Follow all, its feeds and its people.
    pub(super) fn pack_view<'a>(&'a self, uri: &'a AtUri) -> Element<'a, Message> {
        let page = self.lists.pack_view(uri, self, &self.my_did);
        self.scroll(page.map(Message::Lists))
    }

    /// Your lists and starter packs, and buttons to make new ones.
    pub(super) fn my_lists_view(&self) -> Element<'_, Message> {
        let mine = self.profile.lists(&self.my_did);
        let page = self.lists.my_lists_view(mine, self, &self.my_did);
        self.scroll(page.map(Message::Lists))
    }

    /// Someone's starter packs, as cards.
    pub(super) fn pack_cards<'a>(
        &'a self,
        actor: &str,
        empty: &'static str,
    ) -> Element<'a, Message> {
        self.lists.pack_cards(actor, empty).map(Message::Lists)
    }

    /// Making or editing a list or starter pack, with your saved feeds to pick from.
    pub(super) fn list_editor<'a>(&'a self, e: &'a Editor) -> Element<'a, Message> {
        let saved: Vec<(&AtUri, &str)> = self
            .prefs
            .saved_feeds
            .iter()
            .filter_map(|f| match &f.source {
                FeedSource::Generator(u) => Some(u),
                _ => None,
            })
            .map(|u| {
                let name = self
                    .feeds
                    .name(&FeedSource::Generator(u.clone()))
                    .unwrap_or("Feed");
                (u, name)
            })
            .collect();
        lists::editor(e, &saved).map(Message::Lists)
    }

    /// "Add to lists": your lists, ticked where the person is on them.
    pub(super) fn add_to_dialog(a: &AddTo) -> Element<'_, Message> {
        lists::add_to(a).map(Message::Lists)
    }
}

/// A list as a card that opens it.
pub(super) fn list_card<'a>(app: &'a App, l: &'a ListInfo) -> Element<'a, Message> {
    lists::list_card(app, l).map(Message::Lists)
}
