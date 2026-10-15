//! Moving between screens: the Back stack, what each screen loads when it is shown
//! or refreshed, where it was scrolled to, and links.

use super::{
    App, LinkTarget, Message, ProfileTab, Route, TimelineKey, feeds, live, notifications, people,
    search, timeline,
};
use crate::model::{AtUri, FeedSource, PeopleKind};
use iced::Task;
use iced::widget::scrollable;

#[derive(Debug, Clone)]
pub enum Msg {
    Navigate(Route),
    /// Open a screen on top of the current one.
    OpenRoute(Route),
    Back,
    OpenThread(AtUri),
    OpenProfile(String),
    OpenPeople(String, PeopleKind),
    OpenFeed(FeedSource, String),
    Refresh,
    /// Try again after something failed to load.
    Retry,
    /// A scrollable moved; the string says which screen it belongs to.
    Scrolled(String, scrollable::Viewport),
    /// Check the open timeline for newer posts without changing it.
    CheckNew,
    /// Put the newer posts found by a check on top and scroll up to them.
    ShowNew,
    Link(LinkTarget),
    /// Opens a link: in BiFrost when it is somewhere BiFrost can show, otherwise in
    /// the browser.
    OpenUrl(String),
    /// Opens a link in the browser.
    Browse(String),
}

impl App {
    pub(super) fn nav_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Navigate(route) => {
                self.stash_composer();
                if self.routes.len() == 1 && self.route() == &route {
                    // The sidebar item for the screen already open: back to the top.
                    return self.handle(Message::Nav(Msg::ShowNew));
                }
                self.routes = vec![route];
                self.trim_caches();
                Task::batch([self.on_enter(), self.restore_scroll()])
            }
            Msg::Back => {
                if self.composer.is_some() {
                    self.stash_composer();
                } else if self.routes.len() > 1 {
                    self.routes.pop();
                    return Task::batch([self.on_enter(), self.restore_scroll()]);
                }
                Task::none()
            }
            Msg::OpenThread(uri) => self.push_route(Route::Thread(uri)),
            Msg::OpenProfile(actor) => self.push_route(Route::Profile(actor, ProfileTab::Posts)),
            Msg::OpenPeople(actor, kind) => self.push_route(Route::People(actor, kind)),
            Msg::OpenFeed(source, name) => self.push_route(Route::Feed(source, name)),
            Msg::OpenRoute(route) => self.push_route(route),
            Msg::Refresh => self.refresh_current(),
            Msg::Retry if self.deck_active() => self.deck_refresh(),
            Msg::Retry => {
                // A failed "load more" keeps what is loaded and tries the next page again.
                if let Some(key) = self.current_timeline_key()
                    && self.timelines.retry_more(&key)
                {
                    return self.load_more();
                }
                match self.route().clone() {
                    Route::Thread(uri) => self.threads.forget_failed(&uri),
                    Route::Profile(actor, _) => {
                        self.profile.forget_failed(&actor);
                    }
                    _ => {}
                }
                self.on_enter()
            }
            Msg::Scrolled(key, viewport) => {
                let y = viewport.absolute_offset().y;
                self.scroll_offsets.insert(key.clone(), y);
                self.scrolled_to(&key, y);
                if key == self.scroll_key() && viewport.relative_offset().y > 0.85 {
                    self.load_more()
                } else {
                    Task::none()
                }
            }
            Msg::CheckNew if self.deck_active() => self.deck_check_new(),
            Msg::CheckNew => {
                let Some(key) = self.current_timeline_key() else {
                    return Task::none();
                };
                self.timeline_update(timeline::Msg::Peek(key))
            }
            Msg::ShowNew => {
                let Some(key) = self.current_timeline_key() else {
                    return Task::batch([self.refresh_current(), self.scroll_to_top()]);
                };
                match self.show_pending(&key) {
                    Some(images) => Task::batch([images, self.scroll_to_top()]),
                    None => Task::batch([self.refresh_current(), self.scroll_to_top()]),
                }
            }
            Msg::Link(LinkTarget::Url(url)) | Msg::OpenUrl(url) => {
                match self.streamplace_link(&url) {
                    Some(actor) => self.push_route(Route::Channel(actor)),
                    None => self.handle(Message::Nav(Msg::Browse(url))),
                }
            }
            Msg::Browse(url) => {
                // Only web links: post text is untrusted and must not launch local programs.
                if (url.starts_with("https://") || url.starts_with("http://"))
                    && let Err(e) = open::that_detached(&url)
                {
                    self.toast = Some(format!("Could not open the link: {e}"));
                }
                Task::none()
            }
            Msg::Link(LinkTarget::Disguised { shown, url }) => {
                self.popups.confirm_link = Some((shown, url));
                Task::none()
            }
            Msg::Link(LinkTarget::Profile(did)) => self.handle(Message::Nav(Msg::OpenProfile(did))),
            // A cashtag keeps its `$`; a hashtag's `#` is not part of its tag.
            Msg::Link(LinkTarget::Tag(tag)) if tag.starts_with('$') => {
                self.search_update(search::Msg::For(tag))
            }
            Msg::Link(LinkTarget::Tag(tag)) => {
                self.search_update(search::Msg::For(format!("#{tag}")))
            }
        }
    }

    /// Names the scroll position of the current screen. Each feed tab has its own.
    pub(crate) fn scroll_key(&self) -> String {
        match (self.route(), self.feeds.active()) {
            (Route::Home, Some(tab)) => format!("Home:{:?}", tab.source),
            (route, _) => route.scroll_id(),
        }
    }

    /// Puts the current screen back where it was last scrolled to, or at the top.
    pub(super) fn restore_scroll(&self) -> Task<Message> {
        if self.deck_active() {
            return self.restore_deck_scroll();
        }
        let key = self.scroll_key();
        let y = self.scroll_offsets.get(&key).copied().unwrap_or(0.0);
        iced::widget::operation::scroll_to(
            key,
            scrollable::AbsoluteOffset {
                x: None,
                y: Some(y),
            },
        )
    }

    pub(super) fn scroll_to_top(&mut self) -> Task<Message> {
        self.scroll_offsets.remove(&self.scroll_key());
        self.restore_scroll()
    }

    pub(super) fn current_timeline_key(&self) -> Option<TimelineKey> {
        match self.route() {
            Route::Home => self
                .feeds
                .active()
                .map(|t| TimelineKey::Feed(t.source.clone())),
            Route::Profile(did, tab) => tab.filter().map(|f| TimelineKey::Author(did.clone(), f)),
            Route::Feed(source, _) => Some(TimelineKey::Feed(source.clone())),
            Route::Bookmarks => Some(TimelineKey::Bookmarks),
            Route::List(uri) => self.lists.timeline(uri).map(TimelineKey::Feed),
            _ => None,
        }
    }

    /// Loads whatever the newly shown route needs, if it is not cached yet.
    pub(super) fn on_enter(&mut self) -> Task<Message> {
        self.popups.menu = None;
        self.popups.filter = None;
        self.selected = None;
        // A video stops when its screen is left, unless it fills the window.
        if self.video.as_ref().is_some_and(|v| !v.expanded) {
            self.video = None;
        }
        match self.route().clone() {
            Route::Home if self.settings.deck => self.deck_enter(),
            Route::Home => self.load_current(false),
            Route::Notifications => self.open_notifications(false),
            Route::Thread(uri) => {
                if self.threads.contains(&uri) {
                    Task::none()
                } else {
                    self.load_thread(uri)
                }
            }
            Route::Profile(actor, tab) => self.open_profile(actor, tab),
            Route::People(actor, kind) => self.people_update(people::Msg::Open(actor, kind)),
            Route::Feed(..) | Route::Bookmarks => self.load_current(false),
            Route::Feeds => self.feeds_update(feeds::Msg::Load { force: false }),
            Route::Search if !self.search.searched() => self.load_explore(false),
            Route::Settings => Task::batch([
                self.translate_update(super::translate::Msg::ReadKeys),
                self.privacy_update(super::privacy::Msg::Load),
                self.chat_enter(),
            ]),
            Route::Chats | Route::Chat(_) => self.chat_enter(),
            Route::List(uri) => Task::batch([self.ensure_list(uri), self.load_current(false)]),
            Route::StarterPack(uri) => self.load_pack(uri),
            Route::Lists => self.load_lists_screen(),
            Route::Live => self.live_update(live::Msg::OpenBoard),
            Route::Channel(actor) => self.live_update(live::Msg::Open(actor)),
            Route::Atstore => self.atstore_enter(),
            Route::Listing(uri) => self.listing_enter(uri),
            Route::Articles => self.articles_enter(),
            Route::Collection(id) => self.collections_update(super::collections::Msg::Load(id)),
            Route::Collections | Route::Offline => Task::none(),
            Route::Stats => self.stats_enter(),
            Route::Article(a) => self.articles_update(super::articles::Msg::Open(a)),
            Route::Search | Route::Drafts | Route::Moderation => Task::none(),
        }
    }

    pub(super) fn refresh_current(&mut self) -> Task<Message> {
        match self.route().clone() {
            Route::Home if self.settings.deck => self.deck_refresh(),
            Route::Home | Route::Feed(..) | Route::Bookmarks => self.load_current(true),
            Route::Drafts => {
                self.drafts.list = self.drafts.store.list(&self.my_did);
                Task::none()
            }
            Route::Profile(actor, tab) => {
                self.profile.forget(&actor, tab);
                if tab == ProfileTab::StarterPacks {
                    self.lists.actor_packs.remove(&actor);
                }
                Task::batch([self.on_enter(), self.load_current(true)])
            }
            Route::People(actor, kind) => self.people_update(people::Msg::Refresh(actor, kind)),
            Route::Notifications => {
                self.notifications_update(notifications::Msg::Load { append: false })
            }
            Route::Thread(uri) => self.load_thread(uri),
            Route::Search if !self.search.searched() => self.load_explore(true),
            Route::Search => self.search_update(search::Msg::Submit),
            Route::Feeds => self.feeds_update(feeds::Msg::Load { force: true }),
            Route::Chats | Route::Chat(_) => self.chat_refresh(),
            Route::List(uri) => {
                self.lists.screens.remove(&uri);
                Task::batch([self.ensure_list(uri), self.load_current(true)])
            }
            Route::StarterPack(uri) => {
                self.lists.packs.remove(&uri);
                self.load_pack(uri)
            }
            Route::Lists => {
                let me = self.my_did.to_string();
                self.profile.forget_lists(&me);
                self.lists.actor_packs.remove(&me);
                self.load_lists_screen()
            }
            Route::Live => self.live_update(live::Msg::RefreshBoard),
            Route::Channel(actor) => self.live_update(live::Msg::Refresh(actor)),
            Route::Atstore => self.atstore_update(super::atstore::Msg::Refresh),
            Route::Listing(uri) => self.atstore_update(super::atstore::Msg::Reload(uri)),
            Route::Articles => self.articles_update(super::articles::Msg::Refresh),
            Route::Collection(id) => self.collections_update(super::collections::Msg::Load(id)),
            Route::Collections | Route::Offline => Task::none(),
            Route::Stats => self.stats_enter(),
            Route::Article(a) => self.articles_update(super::articles::Msg::Open(a)),
            Route::Settings | Route::Moderation => Task::none(),
        }
    }

    /// Opens a screen on top of the current one, starting at its top.
    pub(super) fn push_route(&mut self, route: Route) -> Task<Message> {
        self.stash_composer();
        self.routes.push(route);
        self.scroll_offsets.remove(&self.scroll_key());
        self.trim_caches();
        Task::batch([self.on_enter(), self.restore_scroll()])
    }

    pub(super) fn load_more(&mut self) -> Task<Message> {
        if let Route::People(actor, kind) = self.route().clone() {
            return self.people_update(people::Msg::More(actor, kind));
        }
        match self.route() {
            Route::Notifications => self.more_notifications(),
            Route::Search => self.search_update(search::Msg::More),
            _ => match self.current_timeline_key() {
                Some(key) => self.load_more_key(key),
                None => Task::none(),
            },
        }
    }
}
