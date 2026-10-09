//! The app's side of deck mode (`features::deck`): keeping the columns with the
//! settings, loading what each column shows, and drawing the deck with the
//! timelines and notifications the app draws.

use super::{App, AuthorFilter, Message, Route, TimelineKey, timeline};
use crate::features::deck::{AddColumn, Event, Host, View, scroll_id, starting};
pub use crate::features::deck::{Deck, Msg};
pub use crate::model::DeckColumn;
use crate::model::FeedSource;
use crate::moderation::Context;
use iced::widget::scrollable;
use iced::{Element, Task};

/// The timeline a column shows, if it shows one.
pub fn column_key(c: &DeckColumn) -> Option<TimelineKey> {
    Some(match c {
        DeckColumn::Following => TimelineKey::Feed(FeedSource::Following),
        DeckColumn::Feed { uri, .. } => TimelineKey::Feed(FeedSource::Generator(uri.clone())),
        DeckColumn::List { uri, .. } => TimelineKey::Feed(FeedSource::List(uri.clone())),
        DeckColumn::Bookmarks => TimelineKey::Bookmarks,
        DeckColumn::Profile { did, .. } => {
            TimelineKey::Author(did.to_string(), AuthorFilter::Posts)
        }
        DeckColumn::Notifications => return None,
    })
}

impl App {
    /// Home is showing columns.
    pub(super) fn deck_active(&self) -> bool {
        self.settings.deck && matches!(self.route(), Route::Home)
    }

    /// This account's columns: the saved ones, or a starting set made from the pinned
    /// feeds.
    pub(super) fn deck_columns(&self) -> Vec<DeckColumn> {
        if let Some(cols) = self.settings.deck_columns.get(&self.my_did) {
            return cols.clone();
        }
        starting(
            self.feeds
                .tabs()
                .iter()
                .map(|t| (&t.source, t.name.as_str())),
        )
    }

    /// The timelines deck columns show, so they are kept when caches are trimmed.
    pub(super) fn deck_keys(&self) -> Vec<TimelineKey> {
        if !self.settings.deck {
            return Vec::new();
        }
        self.deck_columns().iter().filter_map(column_key).collect()
    }

    /// Loads whatever the columns need that is not loaded yet.
    pub(super) fn deck_enter(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();
        for c in self.deck_columns() {
            tasks.push(self.load_column(&c, false));
        }
        Task::batch(tasks)
    }

    pub(super) fn deck_refresh(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();
        for c in self.deck_columns() {
            tasks.push(self.load_column(&c, true));
        }
        Task::batch(tasks)
    }

    fn load_column(&mut self, c: &DeckColumn, force: bool) -> Task<Message> {
        match column_key(c) {
            Some(key) => self.load_key(key, force),
            None if force || self.notifications.is_stale() => self.open_notifications(true),
            None => Task::none(),
        }
    }

    /// Looks for new posts in every column, for the "new posts" buttons.
    pub(super) fn deck_check_new(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();
        for key in self.deck_keys() {
            tasks.push(self.timeline_update(timeline::Msg::Peek(key)));
        }
        Task::batch(tasks)
    }

    /// Whether a loaded timeline belongs to a column on screen.
    pub(super) fn in_deck(&self, key: &TimelineKey) -> bool {
        self.deck_active() && self.deck_keys().contains(key)
    }

    /// Handles one deck message and does what it asks of the app, in order.
    pub(super) fn deck_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.deck.update(msg, &self.deck_columns());
        let mut tasks = vec![out.task.map(Message::Deck)];
        for event in out.events {
            tasks.push(match event {
                Event::Toggle(on) => {
                    self.settings.deck = on;
                    self.save_settings();
                    if !matches!(self.route(), Route::Home) {
                        Task::none()
                    } else if on {
                        self.deck_enter()
                    } else {
                        self.load_current(false)
                    }
                }
                Event::Width(w) => {
                    self.settings.deck_width = w;
                    self.save_settings();
                    Task::none()
                }
                Event::Columns(cols) => {
                    self.settings.deck_columns.insert(self.my_did.clone(), cols);
                    self.save_settings();
                    Task::none()
                }
                Event::Load(c, force) => self.load_column(&c, force),
                Event::More(c) => match column_key(&c) {
                    Some(key) => self.load_more_key(key),
                    None => self.more_notifications(),
                },
                Event::ShowNew(c) => match column_key(&c) {
                    None => Task::none(),
                    Some(key) => match self.show_pending(&key) {
                        Some(images) => Task::batch([images, self.column_to_top(&c)]),
                        None => self.load_column(&c, true),
                    },
                },
                Event::Top(c) => self.column_to_top(&c),
                Event::Scrolled(c, y) => {
                    let id = scroll_id(&c);
                    self.scroll_offsets.insert(id.clone(), y);
                    self.scrolled_to(&id, y);
                    Task::none()
                }
                Event::LoadLists => self.load_my_lists(),
                Event::Toast(t) => {
                    self.toast = Some(t);
                    Task::none()
                }
            });
        }
        Task::batch(tasks)
    }

    /// Puts every column back where it was scrolled to, after coming back to Home.
    pub(super) fn restore_deck_scroll(&self) -> Task<Message> {
        Task::batch(self.deck_columns().iter().map(|c| {
            let id = scroll_id(c);
            let y = self.scroll_offsets.get(&id).copied().unwrap_or(0.0);
            iced::widget::operation::scroll_to(
                id,
                scrollable::AbsoluteOffset {
                    x: None,
                    y: Some(y),
                },
            )
        }))
    }

    pub(super) fn column_to_top(&mut self, c: &DeckColumn) -> Task<Message> {
        let id = scroll_id(c);
        self.scroll_offsets.remove(&id);
        iced::widget::operation::scroll_to(
            id,
            scrollable::AbsoluteOffset {
                x: None,
                y: Some(0.0),
            },
        )
    }

    /// Home as columns.
    pub(super) fn deck_view(&self) -> Element<'_, Message> {
        View {
            columns: self.deck_columns(),
            width: self.settings.deck_width,
            unread: self.notifications.unread(),
            host: Columns { app: self },
            lift: Message::Deck,
        }
        .view()
    }

    /// Choosing a new column.
    pub(super) fn add_column_dialog(&self) -> Element<'_, Message> {
        let lists = match self.profile.lists(&self.my_did) {
            Some(Ok(lists)) => lists.as_slice(),
            _ => &[],
        };
        AddColumn {
            have: self.deck_columns(),
            me: self.me.as_ref().map(|m| &m.did),
            pinned: self
                .feeds
                .tabs()
                .iter()
                .map(|t| (&t.source, t.name.as_str()))
                .collect(),
            lists,
            width: self.settings.deck_width,
        }
        .view()
        .map(Message::Deck)
    }
}

/// The app drawing what deck columns show.
struct Columns<'a> {
    app: &'a App,
}

impl<'a> Host<'a, Message> for Columns<'a> {
    fn body(&self, c: &DeckColumn, scroll_id: &str) -> Element<'a, Message> {
        match column_key(c) {
            Some(key) => {
                let ctx = if matches!(c, DeckColumn::Profile { .. }) {
                    Context::Profile
                } else {
                    Context::Feed
                };
                self.app.timeline_view(Some(&key), ctx, scroll_id)
            }
            None => self.app.notifications_body(scroll_id),
        }
    }

    fn buttons(&self, c: &DeckColumn) -> Option<Element<'a, Message>> {
        match column_key(c)? {
            TimelineKey::Feed(source) => Some(self.app.filter_button(&source, true)),
            _ => None,
        }
    }

    fn pending(&self, c: &DeckColumn) -> usize {
        column_key(c)
            .and_then(|k| self.app.timelines.get(&k))
            .map_or(0, timeline::Timeline::pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_show_their_timelines() {
        let feed = DeckColumn::Feed {
            uri: "at://did:plc:a/app.bsky.feed.generator/x".into(),
            name: "Cats".into(),
        };
        assert_eq!(
            column_key(&feed),
            Some(TimelineKey::Feed(FeedSource::Generator(
                "at://did:plc:a/app.bsky.feed.generator/x".into()
            )))
        );
        assert_eq!(column_key(&DeckColumn::Notifications), None);
    }
}
