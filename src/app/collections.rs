//! Collections (`features::collections`): keeping them on this computer, the
//! Collections screen, a collection's page, and the post menu's part.

use super::pictures::post_images;
use super::post::{self, Opts};
use super::{App, Message, Route, nav};
use crate::components::{self as ui, Icon};
use crate::features::collections::{Event, Host, List, Page, Screen};
use crate::model::{Collection, PostView};
use crate::storage::collections::Collections as Store;
use iced::{Element, Task};

pub use crate::features::collections::Msg;

/// Posts menus list at most this many collections.
const IN_MENU: usize = 6;

#[derive(Default)]
pub(super) struct Collections {
    pub(super) store: Store,
    pub(super) screen: Screen,
}

impl Collections {
    pub(super) fn load() -> Self {
        Self {
            store: Store::load(),
            screen: Screen::default(),
        }
    }
}

impl App {
    fn my_collections(&self) -> &[Collection] {
        self.collections.store.list(&self.my_did)
    }

    pub(super) fn collections_update(&mut self, msg: Msg) -> Task<Message> {
        let all = self.collections.store.list(&self.my_did).to_vec();
        let out = self.collections.screen.update(msg, &all, &self.client);
        let mut tasks = vec![out.task.map(Message::Collections)];
        for event in out.events {
            let me = self.my_did.clone();
            let store = &mut self.collections.store;
            tasks.push(match event {
                Event::Create(name, first) => {
                    let id = store.create(&me, &name, first);
                    store.save();
                    match id {
                        Some(id) => self.handle(open(id)),
                        None => Task::none(),
                    }
                }
                Event::Delete(id) => {
                    store.delete(&me, id);
                    store.save();
                    if self.route() == &Route::Collection(id) {
                        self.handle(Message::Nav(nav::Msg::Back))
                    } else {
                        Task::none()
                    }
                }
                Event::Toggle(id, uri) => {
                    self.popups.menu = None;
                    let added = store.toggle(&me, id, &uri);
                    store.save();
                    let name = self
                        .my_collections()
                        .iter()
                        .find(|c| c.id == id)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    self.toast = Some(if added {
                        format!("Added to {name}.")
                    } else {
                        format!("Removed from {name}.")
                    });
                    Task::none()
                }
                Event::Show => {
                    self.popups.menu = None;
                    self.handle(Message::Nav(nav::Msg::OpenRoute(Route::Collections)))
                }
                Event::Open(id) => self.handle(open(id)),
                Event::Posts(uris) => {
                    let mut pictures = Vec::new();
                    for uri in &uris {
                        if let Some(p) = self.find_post(uri) {
                            post_images(&p, &mut pictures);
                            self.integration_images(&p, &mut pictures);
                        }
                    }
                    self.request_images(pictures)
                }
            });
        }
        Task::batch(tasks)
    }

    pub(super) fn collections_view(&self) -> Element<'_, Message> {
        let list = List {
            screen: &self.collections.screen,
            collections: self.my_collections(),
        };
        self.scroll(list.view().map(Message::Collections))
    }

    pub(super) fn collection_view(&self, id: u64) -> Element<'_, Message> {
        let Some(collection) = self.my_collections().iter().find(|c| c.id == id) else {
            return ui::status("This collection was deleted.");
        };
        let page = Page {
            screen: &self.collections.screen,
            collection,
            host: self,
            lift: Message::Collections,
        };
        self.scroll(page.view())
    }

    pub(super) fn collection_title(&self, id: u64) -> String {
        self.my_collections()
            .iter()
            .find(|c| c.id == id)
            .map_or_else(|| "Collection".into(), |c| c.name.clone())
    }

    /// The post menu's collections: one check per collection, and a new one.
    pub(super) fn collection_items<'a>(&'a self, post: &PostView) -> Vec<Element<'a, Message>> {
        let mut items = vec![ui::menu_divider()];
        for c in self.my_collections().iter().take(IN_MENU) {
            items.push(ui::check_item(
                &c.name,
                c.has(&post.uri),
                Message::Collections(Msg::Toggle(c.id, post.uri.clone())),
            ));
        }
        items.push(ui::menu_item(
            Icon::Folder,
            "New collection…",
            Some(Message::Collections(Msg::NewWith(post.uri.clone()))),
        ));
        items
    }
}

impl Host<Message> for App {
    fn post<'a>(&'a self, p: &'a PostView) -> Element<'a, Message> {
        post::view(self, p, Opts::default())
    }
}

/// Opens a collection's page.
fn open(id: u64) -> Message {
    Message::Nav(nav::Msg::OpenRoute(Route::Collection(id)))
}
