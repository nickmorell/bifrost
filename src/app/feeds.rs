//! The app's side of your feeds (`features::feeds`): passing them messages, doing
//! what their events ask, and placing the Home tabs and the Feeds screen.

use super::{App, Message, Route, account, nav, prefs};
use crate::features::feeds::{Event, Screen, Tabs};

pub use crate::features::feeds::{Feeds, Msg};
use crate::model::FeedGenerator;
use iced::{Element, Task};

impl App {
    pub(super) fn feeds_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self
            .feeds
            .update(msg, &self.prefs.saved_feeds, &self.client);
        let mut tasks = vec![out.task.map(Message::Feeds)];
        for event in out.events {
            tasks.push(match event {
                Event::Pictures(urls) => self.request_images(urls),
                Event::Edit(edit) => self.handle(prefs::edit(edit)),
                Event::Open(source, name) => {
                    self.handle(Message::Nav(nav::Msg::OpenFeed(source, name)))
                }
                Event::Home => self.handle(Message::Nav(nav::Msg::Navigate(Route::Home))),
                Event::Discover => self.handle(Message::Nav(nav::Msg::Navigate(Route::Feeds))),
                Event::Selected => {
                    self.selected = None;
                    Task::batch([self.load_current(false), self.restore_scroll()])
                }
                Event::ShowNew => self.handle(Message::Nav(nav::Msg::ShowNew)),
                Event::SignInAgain => self.handle(Message::Account(account::Msg::Changed(Ok(())))),
            });
        }
        Task::batch(tasks)
    }

    /// The Feeds screen.
    pub(super) fn feeds_view(&self) -> Element<'_, Message> {
        let screen = Screen {
            feeds: &self.feeds,
            saved: &self.prefs.saved_feeds,
            pictures: self,
        };
        self.scroll(screen.view().map(Message::Feeds))
    }

    /// The Home tabs.
    pub(super) fn feed_tabs_view(&self) -> Element<'_, Message> {
        Tabs { feeds: &self.feeds }.view().map(Message::Feeds)
    }

    /// A feed you might add, with a Pin button.
    pub(super) fn discover_row<'a>(&'a self, g: &'a FeedGenerator) -> Element<'a, Message> {
        crate::features::feeds::discover(self, &self.prefs.saved_feeds, g).map(Message::Feeds)
    }
}
