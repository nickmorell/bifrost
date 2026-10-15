//! The app's side of ATstore (`features::atstore`): passing it messages, keeping its
//! directory on this computer, and placing its views on screens.

use super::{App, Message, Route, nav};
use crate::features::atstore::{Event, View};
use crate::integrations::atstore::ID;
use crate::model::AtUri;
use crate::storage::atstore as kept;
use iced::{Element, Task};

pub use crate::features::atstore::{Msg, Store};

impl App {
    /// Whether ATstore is turned on in Settings.
    pub(super) fn atstore_on(&self) -> bool {
        self.settings.integration_enabled(ID)
    }

    pub(super) fn atstore_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.atstore.update(msg, &self.client);
        let mut tasks = vec![out.task.map(Message::Atstore)];
        for event in out.events {
            tasks.push(match event {
                Event::Pictures(urls) => self.request_images(urls),
                Event::Save(catalog) => {
                    kept::save(&catalog);
                    Task::none()
                }
                Event::OpenListing(uri) => self.push_route(Route::Listing(uri)),
                Event::OpenUrl(url) => self.handle(Message::Nav(nav::Msg::OpenUrl(url))),
                Event::OpenProfile(handle) => {
                    self.handle(Message::Nav(nav::Msg::OpenProfile(handle)))
                }
            });
        }
        Task::batch(tasks)
    }

    /// The directory kept on this computer, read the first time it is wanted.
    fn atstore_restore(&mut self) {
        if !self.atstore.has_catalog() {
            self.atstore.restore(kept::load());
        }
    }

    /// Opening the ATstore screen.
    pub(super) fn atstore_enter(&mut self) -> Task<Message> {
        self.atstore_restore();
        self.atstore_update(Msg::Open {
            now: chrono::Utc::now().to_rfc3339(),
            offline: self.offline,
            data_saver: self.settings.data_saver,
        })
    }

    /// Opening a listing's screen.
    pub(super) fn listing_enter(&mut self, uri: AtUri) -> Task<Message> {
        self.atstore_restore();
        let offline = self.offline;
        self.atstore_update(Msg::Show { uri, offline })
    }

    pub(super) fn listing_title(&self, uri: &AtUri) -> String {
        self.atstore.title(uri)
    }

    fn atstore_views(&self) -> View<'_, Self> {
        View {
            store: &self.atstore,
            pictures: self,
            data_saver: self.settings.data_saver,
        }
    }

    pub(super) fn atstore_view(&self) -> Element<'_, Message> {
        self.scroll(self.atstore_views().browse().map(Message::Atstore))
    }

    pub(super) fn listing_view<'a>(&'a self, uri: &'a AtUri) -> Element<'a, Message> {
        self.scroll(self.atstore_views().listing(uri).map(Message::Atstore))
    }
}
