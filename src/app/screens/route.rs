//! The screen for the current route.

use crate::app::{App, Message, Route, TimelineKey, nav, prefs};
use crate::components::{self as ui, Icon, ghost, icon, icon_button, status};
use crate::model::{FeedSource, PeopleKind, PrefEdit};
use crate::moderation::Context;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Row, column, container, row, space, stack, text};
use iced::{Alignment, Element, Length, Padding};

impl App {
    pub(super) fn route_view(&self) -> Element<'_, Message> {
        let route = self.route();
        let title: String = match route {
            Route::Home => "Home".into(),
            Route::Notifications => "Notifications".into(),
            Route::Search => "Search".into(),
            Route::Settings => "Settings".into(),
            Route::Thread(_) => "Post".into(),
            Route::Profile(actor, _) => match self.profile.get(actor) {
                Some(Ok(p)) => p.name().to_string(),
                _ => "Profile".into(),
            },
            Route::People(_, PeopleKind::Followers) => "Followers".into(),
            Route::People(_, PeopleKind::Following) => "Following".into(),
            Route::People(_, PeopleKind::Muted) => "Muted accounts".into(),
            Route::People(_, PeopleKind::Blocked) => "Blocked accounts".into(),
            Route::Moderation => "Moderation".into(),
            Route::Feeds => "Feeds".into(),
            Route::Feed(_, name) => name.clone(),
            Route::Bookmarks => "Bookmarks".into(),
            Route::Drafts => "Drafts".into(),
            Route::Chats => "Chats".into(),
            Route::Chat(id) => self.chat_title(id),
            Route::List(uri) => self.lists.title(uri),
            Route::StarterPack(_) => "Starter pack".into(),
            Route::Lists => "Lists".into(),
            Route::Live => "Live".into(),
            Route::Channel(actor) => self.live_title(actor),
            Route::Articles => "Articles".into(),
            Route::Article(_) => "Article".into(),
            Route::Collections => "Collections".into(),
            Route::Collection(id) => self.collection_title(*id),
            Route::Offline => "Offline reading".into(),
            Route::Stats => "Your stats".into(),
        };
        let mut bar = Row::new()
            .spacing(gap::S)
            .padding(Padding::from([gap::M, gap::L]).left(gap::XL))
            .align_y(Alignment::Center)
            .height(56);
        if self.routes.len() > 1 {
            bar = bar
                .padding(Padding::from([gap::M, gap::L]).left(gap::M))
                .push(icon_button(Icon::Back, Message::Nav(nav::Msg::Back)));
        }
        bar = bar
            .push(text(title).size(size::HEADING).font(theme::SEMIBOLD))
            .push(space::horizontal());
        if let Route::Feed(source, _) = route {
            let saved = self.prefs.saved_feeds.iter().find(|f| &f.source == source);
            bar = bar.push(match saved {
                Some(f) if f.pinned => ghost(
                    "Unpin",
                    prefs::edit(PrefEdit::PinFeed(source.clone(), false)),
                ),
                Some(_) => ghost(
                    "Pin to Home",
                    prefs::edit(PrefEdit::PinFeed(source.clone(), true)),
                ),
                None => ghost(
                    "Pin to Home",
                    prefs::edit(PrefEdit::SaveFeed(source.clone(), true)),
                ),
            });
        }
        if let Route::List(uri) = route
            && matches!(self.lists.screens.get(uri).and_then(|s| s.list.as_ref()), Some(Ok(l)) if !l.is_mod())
        {
            let source = FeedSource::List(uri.clone());
            let saved = self.prefs.saved_feeds.iter().find(|f| f.source == source);
            bar = bar.push(match saved {
                Some(f) if f.pinned => {
                    ghost("Unpin", prefs::edit(PrefEdit::PinFeed(source, false)))
                }
                Some(_) => ghost("Pin to Home", prefs::edit(PrefEdit::PinFeed(source, true))),
                None => ghost("Pin to Home", prefs::edit(PrefEdit::SaveFeed(source, true))),
            });
        }
        let column = match route {
            Route::Feed(source, name) => {
                Some(crate::app::deck::DeckColumn::for_source(source, name))
            }
            Route::List(uri) => match self.lists.screens.get(uri).and_then(|s| s.list.as_ref()) {
                Some(Ok(l)) if !l.is_mod() => Some(crate::app::deck::DeckColumn::List {
                    uri: uri.clone(),
                    name: l.name.clone(),
                }),
                _ => None,
            },
            _ => None,
        };
        if let Some(c) = column.filter(|_| self.settings.deck) {
            bar = bar.push(ghost(
                "Add column",
                Message::Deck(crate::app::deck::Msg::Add(c)),
            ));
        }
        if let Route::Profile(actor, _) = route
            && (actor == self.my_did.as_str()
                || self.me.as_ref().is_some_and(|m| &m.handle == actor))
        {
            bar = bar.push(ghost(
                "Your stats",
                Message::Nav(nav::Msg::OpenRoute(Route::Stats)),
            ));
        }
        if matches!(route, Route::Bookmarks) {
            bar = bar.push(ghost(
                "Collections",
                Message::Nav(nav::Msg::OpenRoute(Route::Collections)),
            ));
        }
        if matches!(route, Route::Home) {
            bar = bar.push(ui::tip(
                icon_button(
                    Icon::Columns,
                    Message::Deck(crate::app::deck::Msg::Toggle(true)),
                ),
                "Show feeds in columns",
            ));
        }
        if let Route::Chat(id) = route {
            bar = bar.push(self.chat_bar_actions(id));
        }
        if let Some(TimelineKey::Feed(source)) = self.current_timeline_key()
            && !self.deck_active()
        {
            bar = bar.push(self.filter_button(&source, false));
        }
        if !matches!(
            route,
            Route::Settings | Route::Moderation | Route::Collections | Route::Offline
        ) {
            bar = bar.push(icon_button(Icon::Refresh, Message::Nav(nav::Msg::Refresh)));
        }

        let body: Element<'_, Message> = match route {
            Route::Home => self.home_view(),
            Route::Notifications => self.notifications_view(),
            Route::Search => self.search_view(),
            Route::Settings => self.settings_view(),
            Route::Thread(uri) => self.thread_view(uri),
            Route::Profile(actor, tab) => self.profile_view(actor, *tab),
            Route::People(actor, kind) => self.people_view(actor, *kind),
            Route::Moderation => self.moderation_view(),
            Route::Feeds => self.feeds_view(),
            Route::Feed(source, _) => self.timeline_screen(&TimelineKey::Feed(source.clone())),
            Route::Bookmarks => self.timeline_screen(&TimelineKey::Bookmarks),
            Route::Drafts => self.drafts_view(),
            Route::Chats => self.chats_view(),
            Route::Chat(id) => self.chat_view(id),
            Route::List(uri) => self.list_view(uri),
            Route::StarterPack(uri) => self.pack_view(uri),
            Route::Lists => self.my_lists_view(),
            Route::Live => self.live_board_view(),
            Route::Channel(actor) => self.channel_view(actor),
            Route::Articles => self.articles_view(),
            Route::Article(_) => self.article_view(),
            Route::Collections => self.collections_view(),
            Route::Collection(id) => self.collection_view(*id),
            Route::Offline => self.offline_view(),
            Route::Stats => self.stats_view(),
        };
        // The Live screens use the whole width, so their title lines up with them.
        let bar = if matches!(route, Route::Live | Route::Channel(_)) {
            bar.into()
        } else {
            ui::reading(bar)
        };
        match self.offline_banner() {
            Some(banner) => column![bar, banner, body].into(),
            None => column![bar, body].into(),
        }
    }

    /// The scrolling body of a screen, centred at reading width.
    pub(in crate::app) fn scroll<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let key = self.scroll_key();
        ui::scroll(
            container(ui::reading(content)).padding(Padding::ZERO.bottom(gap::XXL + gap::XL)),
        )
        .id(key.clone())
        .on_scroll(move |v| Message::Nav(nav::Msg::Scrolled(key.clone(), v)))
        .into()
    }

    /// A timeline screen, with a "new posts" button floating over its top when a
    /// background check found some.
    fn timeline_screen<'a>(&'a self, key: &TimelineKey) -> Element<'a, Message> {
        let tl = self.timelines.get(key);
        let body = self.scroll(self.timeline_view(Some(key), Context::Feed, &self.scroll_key()));
        let pending = tl.map_or(0, crate::app::timeline::Timeline::pending);
        if pending == 0 {
            return body;
        }
        let label = if tl.is_some_and(crate::app::timeline::Timeline::pending_gap) {
            format!("{pending}+ new posts")
        } else if pending == 1 {
            "1 new post".to_string()
        } else {
            format!("{pending} new posts")
        };
        stack![
            body,
            container(ui::new_posts(label, Message::Nav(nav::Msg::ShowNew)))
                .padding(Padding::ZERO.top(gap::S))
                .center_x(Length::Fill),
        ]
        .into()
    }

    // ---- Home ----------------------------------------------------------------------

    fn home_view(&self) -> Element<'_, Message> {
        let body = match self.feeds.active() {
            Some(t) => self.timeline_screen(&TimelineKey::Feed(t.source.clone())),
            None => self.scroll(status("Loading…")),
        };
        // A wide window has room for columns: suggest them once.
        let hint: Element<'_, Message> =
            if !self.settings.deck_hint_seen && self.layout_width() > 1600.0 {
                ui::reading(
                    container(
                        ui::card(
                            row![
                                icon(Icon::Columns, icon_size::L, Tone::Accent),
                                ui::muted(
                                    "Your window has room for columns: several feeds side by side."
                                )
                                .width(Length::Fill),
                                ui::secondary("Try columns").on_press(Message::DeckHint(true)),
                                icon_button(Icon::Close, Message::DeckHint(false)),
                            ]
                            .spacing(gap::M)
                            .align_y(Alignment::Center),
                        )
                        .padding([gap::S, gap::M]),
                    )
                    .padding(Padding::from([0.0, gap::L]).bottom(gap::S)),
                )
            } else {
                space::vertical().height(0).into()
            };
        column![
            hint,
            self.feed_tabs_view(),
            space::vertical().height(gap::S),
            body,
        ]
        .into()
    }

    // ---- Profile -------------------------------------------------------------------

    // ---- Moderation ----------------------------------------------------------------

    // ---- Drafts --------------------------------------------------------------------

    // ---- Notifications -------------------------------------------------------------

    fn notifications_view(&self) -> Element<'_, Message> {
        self.scroll(self.notifications_body(&Route::Notifications.scroll_id()))
    }
}
