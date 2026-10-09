//! Keeping your place: remembering the post at the top of each feed as you scroll,
//! and going back to it when the feed loads again, even after a restart.

use super::{App, Message, Route, TimelineKey, deck, timeline};
use crate::features::deck::scroll_id;
use crate::storage::places::Places;
use iced::Task;
use iced::widget::{operation, scrollable};
use std::time::{Duration, Instant};

/// How long after finding your place the view keeps it at the top while the posts
/// above it are drawn and their real heights replace the guessed ones.
const HOLD: Duration = Duration::from_secs(4);
/// Saving waits at least this long after the last save while you scroll.
const SAVE_EVERY: Duration = Duration::from_secs(10);

/// Your place, held at the top of a view while the posts around it settle.
#[derive(Debug)]
pub(super) struct Anchor {
    key: TimelineKey,
    item: String,
    scroll: String,
    /// Where it was last scrolled to.
    at: f32,
    until: Instant,
}

/// Where you were reading in each feed, and whether that changed since it was saved.
pub(super) struct Reading {
    pub(super) places: Places,
    pub(super) dirty: bool,
    pub(super) saved: Instant,
    /// Your place, held at the top of a view just after it was found.
    pub(super) anchor: Option<Anchor>,
}

impl Default for Reading {
    fn default() -> Self {
        Self {
            places: Places::load(),
            dirty: false,
            saved: Instant::now(),
            anchor: None,
        }
    }
}

impl App {
    /// Before a feed loads for the first time: look for the post you were reading.
    pub(super) fn find_place(&mut self, key: &TimelineKey) -> Task<Message> {
        let TimelineKey::Feed(source) = key else {
            return Task::none();
        };
        if self.timelines.contains(key) {
            return Task::none();
        }
        match self.reading.places.get(&self.my_did, source.id()) {
            Some(item) => {
                let item = item.to_string();
                self.timeline_update(timeline::Msg::Find(key.clone(), item))
            }
            None => Task::none(),
        }
    }

    /// The post you were reading was found: scroll to it, wherever its feed is shown.
    pub(super) fn go_to_place(&mut self, key: TimelineKey, item: String) -> Task<Message> {
        let Some(scroll) = self.place_scroll(&key) else {
            return Task::none();
        };
        let Some(at) = self.timelines.get(&key).and_then(|t| t.top_of(&item)) else {
            return Task::none();
        };
        self.reading.anchor = Some(Anchor {
            key,
            item,
            scroll: scroll.clone(),
            at,
            until: Instant::now() + HOLD,
        });
        self.scroll_to(scroll, at)
    }

    /// After a post of `key` was measured: keep your place at the top.
    pub(super) fn hold_place(&mut self, key: &TimelineKey) -> Task<Message> {
        let Some(a) = &self.reading.anchor else {
            return Task::none();
        };
        if &a.key != key {
            return Task::none();
        }
        if Instant::now() > a.until {
            self.reading.anchor = None;
            return Task::none();
        }
        let Some(at) = self.timelines.get(key).and_then(|t| t.top_of(&a.item)) else {
            return Task::none();
        };
        if (at - a.at).abs() < 1.0 {
            return Task::none();
        }
        let scroll = a.scroll.clone();
        if let Some(a) = &mut self.reading.anchor {
            a.at = at;
        }
        self.scroll_to(scroll, at)
    }

    fn scroll_to(&mut self, scroll: String, y: f32) -> Task<Message> {
        self.scroll_offsets.insert(scroll.clone(), y);
        operation::scroll_to(
            scroll,
            scrollable::AbsoluteOffset {
                x: None,
                y: Some(y),
            },
        )
    }

    /// A view was scrolled to `y`: remember the post at its top, if it shows a feed.
    pub(super) fn scrolled_to(&mut self, scroll: &str, y: f32) {
        if let Some(a) = &self.reading.anchor
            && a.scroll == scroll
            && (y - a.at).abs() > 2.0
        {
            // Scrolled by hand: stop holding the place.
            self.reading.anchor = None;
        }
        let Some(TimelineKey::Feed(source)) = self.scroll_feed(scroll) else {
            return;
        };
        let key = TimelineKey::Feed(source.clone());
        let Some(item) = self.timelines.get(&key).and_then(|t| t.item_at(y)) else {
            return;
        };
        if self.reading.places.set(&self.my_did, source.id(), &item) {
            self.reading.dirty = true;
            if self.reading.saved.elapsed() > SAVE_EVERY {
                self.save_places(false);
            }
        }
    }

    /// Saves where you were reading, if it changed since the last save: before
    /// returning when BiFrost is about to quit (`now`).
    pub(super) fn save_places(&mut self, now: bool) {
        if std::mem::take(&mut self.reading.dirty) {
            self.reading.places.save(now);
            self.reading.saved = Instant::now();
        }
    }

    /// The view that shows `key` with its posts starting at the top of the scroll
    /// area, as feeds on Home, feed screens and deck columns do.
    fn place_scroll(&self, key: &TimelineKey) -> Option<String> {
        if self.deck_active() {
            return self
                .deck_columns()
                .iter()
                .find(|c| deck::column_key(c).as_ref() == Some(key))
                .map(scroll_id);
        }
        (self.current_timeline_key().as_ref() == Some(key) && self.shows_feed_on_top())
            .then(|| self.scroll_key())
    }

    /// The feed a view shows, when its posts start at the top of the scroll area.
    fn scroll_feed(&self, scroll: &str) -> Option<TimelineKey> {
        if self.deck_active() {
            return self
                .deck_columns()
                .iter()
                .find(|c| scroll_id(c) == scroll)
                .and_then(deck::column_key);
        }
        if scroll != self.scroll_key() || !self.shows_feed_on_top() {
            return None;
        }
        self.current_timeline_key()
    }

    /// The current screen is a feed with nothing above its posts.
    fn shows_feed_on_top(&self) -> bool {
        matches!(self.route(), Route::Home | Route::Feed(..))
    }
}
