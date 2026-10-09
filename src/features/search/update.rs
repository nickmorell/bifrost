//! Running searches, suggesting while typing, paging results, and loading the
//! Explore page.

use super::message::{Event, Msg};
use super::state::{INPUT, Search};
use super::tab::Tab;
use crate::api::ApiError;
use crate::api::service::SearchApi;
use crate::features::Update;
use crate::model::{AtUri, Did, FeedSource, TrendTarget};
use iced::Task;
use std::time::Duration;

type Out = Update<Msg, Event>;

/// How long typing has to pause before people are suggested.
const SUGGEST_DELAY: Duration = Duration::from_millis(250);

impl Search {
    pub fn update(&mut self, msg: Msg, api: &impl SearchApi) -> Out {
        match msg {
            Msg::Input(q) => {
                self.query = q;
                self.typing = true;
                self.suggest_seq += 1;
                let seq = self.suggest_seq;
                let q = self.query.trim();
                if q.is_empty() || q.starts_with('#') {
                    self.suggest.clear();
                    return Update::none();
                }
                // Made when the task runs, inside the runtime that drives its timer.
                let pause = async { tokio::time::sleep(SUGGEST_DELAY).await };
                Update::task(Task::perform(pause, move |()| Msg::Suggest(seq)))
            }
            Msg::Suggest(seq) => {
                let q = self.query.trim().to_string();
                if seq != self.suggest_seq || q.is_empty() {
                    return Update::none();
                }
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.typeahead(&q).await },
                    move |r| Msg::Suggested(seq, r),
                ))
            }
            Msg::Suggested(seq, result) => {
                if seq != self.suggest_seq {
                    return Update::none();
                }
                // A failed suggestion is not worth interrupting typing for.
                let people = result.unwrap_or_default();
                let urls = people.iter().filter_map(|a| a.avatar.clone()).collect();
                self.suggest = people;
                Update::event(Event::Pictures(urls))
            }
            Msg::Submit => self.submit(api),
            Msg::For(q) => {
                self.query = q;
                let out = self.submit(api);
                let mut events = vec![Event::Show];
                events.extend(out.events);
                Update {
                    task: out.task,
                    events,
                }
            }
            Msg::Clear => {
                self.query.clear();
                self.submitted.clear();
                self.typing = false;
                self.suggest.clear();
                self.suggest_seq += 1;
                self.seq += 1;
                Update::task(iced::widget::operation::focus(INPUT)).and(Event::Explore(false))
            }
            Msg::Forget(q) => Update::event(Event::Forget(q)),
            Msg::Tab(tab) => {
                self.tab = Some(tab);
                let empty = match tab {
                    Tab::Top | Tab::Latest => self.posts_sort != Some(tab),
                    Tab::People => self.people.is_empty(),
                    Tab::Feeds => self.feeds.is_empty(),
                    Tab::Saved => false,
                };
                if empty && self.searched() && !self.loading(tab) {
                    Update::task(self.run(tab, false, api))
                } else {
                    Update::none()
                }
            }
            Msg::More => match self.tab {
                Some(tab) if self.loading(tab) => Update::none(),
                Some(tab @ (Tab::Top | Tab::Latest))
                    if self.posts_sort == Some(tab) && self.posts_cursor.is_some() =>
                {
                    Update::task(self.run(tab, true, api))
                }
                Some(Tab::People) if self.people_cursor.is_some() => {
                    Update::task(self.run(Tab::People, true, api))
                }
                Some(Tab::Feeds) if self.feeds_cursor.is_some() => {
                    Update::task(self.run(Tab::Feeds, true, api))
                }
                _ => Update::none(),
            },
            Msg::Posts(seq, sort, append, result) => {
                if seq != self.seq || (append && self.posts_sort != Some(sort)) {
                    return Update::none();
                }
                self.posts_loading = false;
                match result {
                    Ok(page) => {
                        let uris = page.items.iter().map(|p| p.uri.clone()).collect();
                        if !append {
                            self.posts.clear();
                            self.posts_sort = Some(sort);
                        }
                        self.posts.extend(page.items);
                        self.posts_cursor = page.cursor;
                        Update::event(Event::Posts(uris))
                    }
                    Err(e) => Update::event(Event::Failed(e)),
                }
            }
            Msg::People(seq, append, result) => {
                if seq != self.seq {
                    return Update::none();
                }
                self.people_loading = false;
                match result {
                    Ok(page) => {
                        let urls = page.items.iter().filter_map(|a| a.avatar.clone()).collect();
                        if !append {
                            self.people.clear();
                        }
                        self.people.extend(page.items);
                        self.people_cursor = page.cursor;
                        Update::event(Event::Pictures(urls))
                    }
                    Err(e) => Update::event(Event::Failed(e)),
                }
            }
            Msg::Feeds(seq, append, result) => {
                if seq != self.seq {
                    return Update::none();
                }
                self.feeds_loading = false;
                match result {
                    Ok(page) => {
                        let urls = page.items.iter().filter_map(|g| g.avatar.clone()).collect();
                        if !append {
                            self.feeds.clear();
                        }
                        self.feeds.extend(page.items);
                        self.feeds_cursor = page.cursor;
                        Update::event(Event::Pictures(urls))
                    }
                    Err(e) => Update::event(Event::Failed(e)),
                }
            }

            Msg::Explore { force } => {
                let mut tasks = Vec::new();
                if force || !matches!(self.trends, Some(Ok(_))) {
                    let api = api.clone();
                    tasks.push(Task::perform(
                        async move { api.trending().await },
                        Msg::Trends,
                    ));
                }
                if force || self.suggested_people.is_empty() {
                    let api = api.clone();
                    tasks.push(Task::perform(
                        async move { api.suggested_people().await },
                        Msg::SuggestedPeople,
                    ));
                }
                Update::task(Task::batch(tasks))
            }
            Msg::Trends(result) => {
                self.trends = Some(result.map_err(|e| e.friendly()));
                Update::none()
            }
            Msg::SuggestedPeople(result) => {
                let people = result.unwrap_or_default();
                let urls = people.iter().filter_map(|a| a.avatar.clone()).collect();
                self.suggested_people = people;
                Update::event(Event::Pictures(urls))
            }
            Msg::OpenTrend(trend) => match trend.target() {
                TrendTarget::Search(q) => self.update(Msg::For(q), api),
                TrendTarget::Feed(owner, rkey) => {
                    let api = api.clone();
                    let name = trend.name().to_string();
                    Update::task(Task::perform(
                        async move {
                            let did = if owner.starts_with("did:") {
                                Did::from(owner)
                            } else {
                                api.resolve_handle(&owner).await?
                            };
                            Ok::<_, ApiError>(AtUri::new(&did, "app.bsky.feed.generator", &rkey))
                        },
                        move |r| Msg::TrendFeed(name.clone(), r),
                    ))
                }
            },
            Msg::TrendFeed(name, Ok(uri)) => {
                Update::event(Event::OpenFeed(FeedSource::Generator(uri), name))
            }
            Msg::TrendFeed(name, Err(_)) => self.update(Msg::For(name), api),
            Msg::MoreFeeds => Update::event(Event::MoreFeeds),

            Msg::PostWithTag(tag) => Update::event(Event::PostWithTag(tag)),
            Msg::MuteTag(word, mute) => Update::event(Event::MuteTag(word, mute)),
        }
    }

    /// Runs the query in the box from the first page, on the open tab.
    fn submit(&mut self, api: &impl SearchApi) -> Out {
        let q = self.query.trim().to_string();
        if q.is_empty() {
            return Update::none();
        }
        self.query.clone_from(&q);
        self.submitted.clone_from(&q);
        self.typing = false;
        self.suggest.clear();
        self.suggest_seq += 1;
        self.seq += 1;
        self.posts_loading = false;
        self.people_loading = false;
        self.feeds_loading = false;
        self.posts.clear();
        self.posts_sort = None;
        self.people.clear();
        self.feeds.clear();
        self.posts_cursor = None;
        self.people_cursor = None;
        self.feeds_cursor = None;
        let mut tab = self.tab.unwrap_or(Tab::Top);
        // Tag pages only have posts.
        if self.tag().is_some() && matches!(tab, Tab::People | Tab::Feeds) {
            tab = Tab::Top;
        }
        self.tab = Some(tab);
        Update::task(self.run(tab, false, api)).and(Event::Remember(q))
    }

    /// Loads the first page of `tab`, or the next one when `append`.
    fn run(&mut self, tab: Tab, append: bool, api: &impl SearchApi) -> Task<Msg> {
        let api = api.clone();
        let q = self.submitted.clone();
        let seq = self.seq;
        match tab {
            // Searched on this computer as it is drawn.
            Tab::Saved => Task::none(),
            Tab::Top | Tab::Latest => {
                self.posts_loading = true;
                let cursor = if append {
                    self.posts_cursor.clone()
                } else {
                    None
                };
                Task::perform(
                    async move { api.search_posts(&q, tab.sort(), cursor).await },
                    move |r| Msg::Posts(seq, tab, append, r),
                )
            }
            Tab::Feeds => {
                self.feeds_loading = true;
                let cursor = if append {
                    self.feeds_cursor.clone()
                } else {
                    None
                };
                Task::perform(
                    async move { api.search_feeds(&q, cursor).await },
                    move |r| Msg::Feeds(seq, append, r),
                )
            }
            Tab::People => {
                self.people_loading = true;
                let cursor = if append {
                    self.people_cursor.clone()
                } else {
                    None
                };
                Task::perform(
                    async move { api.search_actors(&q, cursor).await },
                    move |r| Msg::People(seq, append, r),
                )
            }
        }
    }
}
