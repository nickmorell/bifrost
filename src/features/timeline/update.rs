//! Loading pages of timelines and fitting them in.

use super::key::{Key, Load};
use super::merge::{Merge, item_key, merge_newest};
use super::message::{Ctx, Event, Msg};
use super::place::Look;
use super::state::{Gap, Timeline, Timelines};
use crate::api::service::TimelineApi;
use crate::features::Update;
use crate::model::{FeedItem, FeedSource, Page};
use iced::Task;
use std::collections::HashSet;

type Out = Update<Msg, Event>;

/// Fewer posts than this may not fill the screen.
const FILL: usize = 8;

impl Timelines {
    pub fn update(&mut self, msg: Msg, ctx: &Ctx, api: &impl TimelineApi) -> Out {
        match msg {
            Msg::Load { key, force } => {
                let tl = self.by_key.entry(key.clone()).or_default();
                if tl.loading || (!force && (!tl.items.is_empty() || tl.error.is_some())) {
                    return Update::none();
                }
                tl.loading = true;
                // A forced load keeps the posts already loaded and adds the new ones on top.
                let load = if force && !tl.items.is_empty() {
                    Load::Refresh
                } else {
                    Load::First
                };
                Update::task(fetch(key, None, load, api))
            }
            Msg::More(key) => self.update_more(key, api),
            Msg::Peek(key) => match self.by_key.get_mut(&key) {
                Some(tl) if !tl.loading && !tl.items.is_empty() => {
                    tl.loading = true;
                    Update::task(fetch(key, None, Load::Peek, api))
                }
                _ => Update::none(),
            },
            Msg::Refresh(key) => {
                let tl = self.by_key.entry(key.clone()).or_default();
                if tl.loading {
                    return Update::none();
                }
                tl.loading = true;
                Update::task(fetch(key, None, Load::Refresh, api))
            }
            Msg::Loaded(key, load, result) => self.loaded(key, load, result, ctx, api),
            Msg::Find(key, item) => {
                let look = self.seek(&key, item);
                self.found(&key, look, api)
            }
            Msg::Measured(key, row, height) => {
                if let Some(tl) = self.by_key.get_mut(&key) {
                    tl.heights.set(row, height);
                }
                Update::none()
            }
        }
    }

    fn loaded(
        &mut self,
        key: Key,
        load: Load,
        mut result: Result<Page<FeedItem>, crate::api::ApiError>,
        ctx: &Ctx,
        api: &impl TimelineApi,
    ) -> Out {
        // Whether the page ran out is decided before anything is filtered out.
        let done = result
            .as_ref()
            .is_ok_and(|p| p.cursor.is_none() || p.items.is_empty());
        let fetched_len = result.as_ref().map_or(0, |p| p.items.len());
        if let (Key::Feed(source), Ok(page)) = (&key, &mut result) {
            if *source == FeedSource::Following {
                page.items.retain(|i| ctx.following.keep(ctx.me, i));
            }
            if let Some(filter) = ctx.filters.get(source.id()) {
                page.items.retain(|i| filter.keep(i));
            }
        }
        let tl = self.by_key.entry(key.clone()).or_default();
        tl.loading = false;
        match result {
            Ok(page) => {
                let added = page.items.len();
                // New posts found in the background get their pictures only if they
                // are shown.
                let fetched = (load != Load::Peek).then(|| page.items.clone());
                tl.fit(load, page, done);
                // Filters left too little to fill the screen: fetch the next page,
                // since scrolling cannot ask for it when there is nothing to scroll.
                let filtered = added < fetched_len;
                let short = match load {
                    Load::First => filtered && tl.items.len() < FILL,
                    Load::More => added == 0 || filtered && tl.items.len() < FILL,
                    Load::Refresh | Load::Peek => false,
                };
                let empty_more = short && !done;
                let mut out = Update::none();
                if let Some(items) = fetched {
                    out = out.and(Event::Fetched(items));
                }
                if empty_more {
                    out = out.and(Event::Empty(key.clone()));
                }
                if matches!(load, Load::First | Load::More) {
                    let look = self.look(&key);
                    let more = self.found(&key, look, api);
                    out.task = Task::batch([out.task, more.task]);
                    out.events.extend(more.events);
                }
                out
            }
            // A failed background check is not worth showing.
            Err(_) if load == Load::Peek => Update::none(),
            Err(e) => {
                tl.error = Some(e.friendly());
                Update::event(Event::Failed(e))
            }
        }
    }
}

impl Timelines {
    /// Acts on looking for the entry you were reading: reports it found, or loads the
    /// next page to look in.
    fn found(&mut self, key: &Key, look: Look, api: &impl TimelineApi) -> Out {
        match look {
            Look::Found(event) => Update::event(event),
            Look::MorePlease => self.update_more(key.clone(), api),
            Look::Nothing => Update::none(),
        }
    }

    /// Loads the next page of `key`, unless one is loading or there are no more.
    fn update_more(&mut self, key: Key, api: &impl TimelineApi) -> Out {
        let tl = self.by_key.entry(key.clone()).or_default();
        if tl.loading || tl.done || tl.cursor.is_none() {
            return Update::none();
        }
        tl.loading = true;
        let cursor = tl.cursor.clone();
        Update::task(fetch(key, cursor, Load::More, api))
    }
}

impl Timeline {
    /// Fits a fetched page in with the posts already loaded.
    fn fit(&mut self, load: Load, page: Page<FeedItem>, done: bool) {
        match load {
            Load::First => {
                self.error = None;
                self.done = done;
                self.cursor = page.cursor;
                self.items = page.items;
                self.pages = vec![(self.items.len(), self.cursor.clone())];
            }
            Load::More => {
                self.error = None;
                self.done = done;
                self.cursor = page.cursor;
                let seen: HashSet<String> = self.items.iter().map(item_key).collect();
                self.items.extend(
                    page.items
                        .into_iter()
                        .filter(|i| !seen.contains(&item_key(i))),
                );
                self.pages.push((self.items.len(), self.cursor.clone()));
            }
            Load::Refresh => {
                self.error = None;
                self.pending.clear();
                self.pending_gap = None;
                match merge_newest(&self.items, page.items) {
                    Merge::Prepend(fresh) if self.items.is_empty() => {
                        self.items = fresh;
                        self.cursor = page.cursor;
                        self.done = done;
                        self.pages = vec![(self.items.len(), self.cursor.clone())];
                    }
                    Merge::Prepend(fresh) => {
                        let n = fresh.len();
                        self.items.splice(0..0, fresh);
                        for (len, _) in &mut self.pages {
                            *len += n;
                        }
                    }
                    Merge::Gap(fresh) => {
                        self.items = fresh;
                        self.cursor = page.cursor;
                        self.done = done;
                        self.pages = vec![(self.items.len(), self.cursor.clone())];
                        self.heights.clear();
                    }
                }
            }
            Load::Peek => match merge_newest(&self.items, page.items) {
                Merge::Prepend(fresh) => {
                    self.pending = fresh;
                    self.pending_gap = None;
                }
                Merge::Gap(fresh) => {
                    self.pending = fresh;
                    self.pending_gap = Some(Gap {
                        cursor: page.cursor,
                    });
                }
            },
        }
    }
}

/// Fetches a page of `key`, continuing from `cursor`.
fn fetch(key: Key, cursor: Option<String>, load: Load, api: &impl TimelineApi) -> Task<Msg> {
    let api = api.clone();
    let k = key.clone();
    Task::perform(
        async move {
            match &k {
                Key::Feed(source) if load == Load::Peek => api.feed_peek(source).await,
                Key::Feed(source) => api.feed(source, cursor).await,
                Key::Author(actor, filter) => api.author_feed(actor, *filter, cursor).await,
                Key::Bookmarks => api.bookmarks(cursor).await,
                Key::Quotes(uri, sort) => api.quotes(uri, *sort, cursor).await,
            }
        },
        move |r| Msg::Loaded(key.clone(), load, r),
    )
}
