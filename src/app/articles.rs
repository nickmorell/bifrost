//! Articles (`features::articles`): the reader, the Articles screen, and the
//! Subscribe buttons on article cards.

use super::{App, Message, Route, nav};
use crate::features::articles::{Event, List, Reader};
use crate::model::ArticleRef;
use iced::{Element, Task};

pub use crate::features::articles::{Articles, Msg};

impl App {
    pub(super) fn articles_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.articles.update(msg, &self.client);
        let mut tasks = vec![out.task.map(Message::Articles)];
        for event in out.events {
            tasks.push(match event {
                Event::Toast(t) => {
                    self.toast = Some(t);
                    Task::none()
                }
                Event::OpenUrl(url) => self.handle(Message::Nav(nav::Msg::OpenUrl(url))),
                Event::Read(article) => self.handle(read(article)),
                Event::Pictures(urls) => self.request_images(urls),
            });
        }
        Task::batch(tasks)
    }

    pub(super) fn article_view(&self) -> Element<'_, Message> {
        let reader = Reader {
            articles: &self.articles,
            pictures: self,
            look: self.look(),
        };
        self.scroll(reader.view().map(Message::Articles))
    }

    pub(super) fn articles_view(&self) -> Element<'_, Message> {
        let list = List {
            articles: &self.articles,
        };
        self.scroll(list.view().map(Message::Articles))
    }

    /// Opening the Articles screen fetches its list the first time.
    pub(super) fn articles_enter(&mut self) -> Task<Message> {
        if self.articles.stale() {
            self.articles_update(Msg::Refresh)
        } else {
            Task::none()
        }
    }
}

/// Opens an article in the reader.
pub(super) fn read(article: ArticleRef) -> Message {
    Message::Nav(nav::Msg::OpenRoute(Route::Article(article)))
}
