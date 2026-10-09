//! What each feed leaves out: the filter button over a feed, its menu, and loading
//! the feed again when its filters change.

use super::{App, Message, TimelineKey, deck};
use crate::components::{self as ui, Icon, icon_toggle, popover};
use crate::features::timeline::filter_menu;
use crate::model::{FeedFilter, FeedSource};
use crate::theme::gap;
use iced::{Element, Task};

#[derive(Debug, Clone)]
pub enum Msg {
    Open(FeedSource),
    Close,
    Set(FeedSource, FeedFilter),
}

impl App {
    pub(super) fn filters_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Open(source) => {
                self.popups.filter = Some(source);
                Task::none()
            }
            Msg::Close => {
                self.popups.filter = None;
                Task::none()
            }
            Msg::Set(source, filter) => {
                let id = source.id().to_string();
                if filter.is_on() {
                    self.settings.feed_filters.insert(id, filter);
                } else {
                    self.settings.feed_filters.remove(&id);
                }
                self.save_settings();
                // What was loaded was filtered as it arrived, so the feed loads again,
                // from the top.
                let key = TimelineKey::Feed(source);
                self.timelines.forget(&key);
                let mut tasks = Vec::new();
                if self.deck_active() {
                    for c in self.deck_columns() {
                        if deck::column_key(&c).as_ref() == Some(&key) {
                            tasks.push(self.column_to_top(&c));
                        }
                    }
                } else if self.current_timeline_key().as_ref() == Some(&key) {
                    tasks.push(self.scroll_to_top());
                }
                if !tasks.is_empty() {
                    tasks.push(self.load_key(key, false));
                }
                Task::batch(tasks)
            }
        }
    }

    /// What `source` leaves out.
    pub(super) fn feed_filter(&self, source: &FeedSource) -> FeedFilter {
        self.settings
            .feed_filters
            .get(source.id())
            .copied()
            .unwrap_or_default()
    }

    /// The button that opens the filters of `source`, tinted while any are on;
    /// `small` for a deck column's header.
    pub(super) fn filter_button<'a>(
        &self,
        source: &FeedSource,
        small: bool,
    ) -> Element<'a, Message> {
        let filter = self.feed_filter(source);
        let mut button = icon_toggle(
            Icon::Filter,
            filter.is_on(),
            Message::Filter(Msg::Open(source.clone())),
        );
        if small {
            button = button.padding(gap::XS + 1.0);
        }
        let button = ui::tip(
            button,
            if filter.is_on() {
                "Filters are on"
            } else {
                "Filter this feed"
            },
        );
        let open = self.popups.filter.as_ref() == Some(source);
        let source = source.clone();
        let menu = open.then(|| {
            filter_menu(filter).map(move |f| Message::Filter(Msg::Set(source.clone(), f)))
        });
        popover(button, menu, Message::Filter(Msg::Close)).into()
    }
}
