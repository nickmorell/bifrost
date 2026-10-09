//! The app's side of search (`features::search`): passing it messages, doing what its
//! events ask, and drawing the search screen with the posts, people and feeds the app
//! draws.

use super::pictures::post_images;
use super::post::{self, Opts};
use super::{App, Message, Route, nav, prefs};
use crate::features::search::{Event, Host, View};

pub use crate::features::search::{INPUT, Msg, Search};
use crate::model::{Author, FeedGenerator, PostView, PrefEdit};
use iced::{Element, Task};

impl App {
    /// Handles one search message and does what it asks of the app, in order.
    pub(super) fn search_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.search.update(msg, &self.client);
        let mut tasks = vec![out.task.map(Message::Search)];
        for event in out.events {
            tasks.push(match event {
                Event::Pictures(urls) => self.request_images(urls),
                Event::Posts(uris) => {
                    let mut pictures = Vec::new();
                    for uri in &uris {
                        if let Some(p) = self.search.post(uri) {
                            post_images(p, &mut pictures);
                            self.integration_images(p, &mut pictures);
                        }
                    }
                    self.request_images(pictures)
                }
                Event::Failed(e) => self.handle_error(e),
                Event::Show => {
                    if self.route() != &Route::Search {
                        self.routes.push(Route::Search);
                    }
                    Task::none()
                }
                Event::Remember(q) => {
                    self.settings.remember_search(&q);
                    self.save_settings();
                    Task::none()
                }
                Event::Forget(q) => {
                    match q {
                        Some(q) => self.settings.recent_searches.retain(|s| *s != q),
                        None => self.settings.recent_searches.clear(),
                    }
                    self.save_settings();
                    Task::none()
                }
                Event::Explore(force) => self.load_explore(force),
                Event::OpenFeed(source, name) => {
                    self.handle(Message::Nav(nav::Msg::OpenFeed(source, name)))
                }
                Event::MoreFeeds => self.handle(Message::Nav(nav::Msg::OpenRoute(Route::Feeds))),
                Event::PostWithTag(tag) => {
                    if self.composer.is_none() {
                        self.composer = Some(
                            self.new_composer(super::Composer::default())
                                .with_text(&format!("\n\n#{tag}")),
                        );
                    }
                    Task::none()
                }
                Event::MuteTag(word, mute) => self.handle(prefs::edit(if mute {
                    PrefEdit::MuteWord(word)
                } else {
                    PrefEdit::UnmuteWord(word)
                })),
            });
        }
        Task::batch(tasks)
    }

    /// Loads what the Explore page shows before anything is searched; `force` loads
    /// it all again.
    pub(super) fn load_explore(&mut self, force: bool) -> Task<Message> {
        let search = self.search_update(Msg::Explore { force });
        let feeds = self.feeds_update(super::feeds::Msg::Suggest { force });
        Task::batch([search, feeds])
    }

    /// The search screen.
    pub(super) fn search_view(&self) -> Element<'_, Message> {
        let muted_tag = self.search.tag().and_then(|tag| {
            self.prefs
                .muted_words
                .iter()
                .find(|w| w.value.eq_ignore_ascii_case(tag))
                .map(|w| w.value.as_str())
        });
        let feeds = self.feeds.suggested();
        View {
            search: &self.search,
            recent: &self.settings.recent_searches,
            tags: &self.settings.recent_tags,
            muted_tag,
            feeds,
            host: self,
            lift: Message::Search,
        }
        .screen()
    }
}

impl Host<Message> for App {
    fn post<'a>(&'a self, p: &'a PostView) -> Element<'a, Message> {
        post::view(self, p, Opts::default())
    }

    fn person<'a>(&'a self, author: &'a Author) -> Element<'a, Message> {
        self.person_row(author)
    }

    fn feed<'a>(&'a self, feed: &'a FeedGenerator) -> Element<'a, Message> {
        self.discover_row(feed)
    }

    fn retry<'a>(&self, why: String) -> Element<'a, Message> {
        super::util::retry(why)
    }

    fn saved<'a>(&'a self, query: &'a str) -> Element<'a, Message> {
        self.saved_search(query)
    }

    fn top_links(&self) -> Option<Element<'_, Message>> {
        self.top_links_view()
    }

    fn scroll<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        App::scroll(self, content)
    }
}
