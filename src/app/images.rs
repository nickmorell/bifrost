//! Downloaded images, kept within a memory budget. The least recently drawn ones are
//! dropped when the budget runs out and fetched again if they come back on screen;
//! failed downloads are retried a couple of times. Entries holding nothing are
//! forgotten once there are many, keeping only a hash of each URL so it is fetched
//! again if it comes back.

use super::media::Frames;
use iced::widget::image::Handle;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, RandomState};
use std::time::{Duration, Instant};

/// Memory held by downloaded images: their decoded pixels.
const BUDGET: usize = 160 * 1024 * 1024;
/// Evicting stops once usage is back under this.
const LOW_WATER: usize = BUDGET / 4 * 3;
/// Entries kept, most of them holding nothing (dropped or failed); past this the
/// least recently drawn of those are forgotten.
const MAX_ENTRIES: usize = 4000;
/// Images drawn this recently are on screen and are never dropped: dropping one only
/// for it to be fetched again at once makes pictures flicker.
const ON_SCREEN: Duration = Duration::from_secs(2);
/// Downloads attempted per image before giving up for the session.
const MAX_TRIES: u8 = 3;
/// Wait before the first retry; doubled for each one after.
const RETRY_AFTER: Duration = Duration::from_secs(10);

pub enum State {
    Loading,
    Ready(Handle),
    Animated(Frames),
    Failed(Instant),
    /// Dropped to save memory; fetched again when next drawn.
    Evicted,
}

struct Entry {
    state: State,
    size: usize,
    /// When it was last drawn, on `ImageCache::clock`.
    used: Cell<u64>,
    /// When it was last drawn.
    drawn: Cell<Option<Instant>>,
    tries: u8,
}

/// The URL actually fetched for `url`. Avatars are drawn at 76 px at most, so they
/// come from Bluesky's thumbnail size rather than the full 1000 px picture.
pub fn sized(url: &str) -> std::borrow::Cow<'_, str> {
    const FULL: &str = "/img/avatar/plain/";
    if url.starts_with("https://cdn.bsky.app/") && url.contains(FULL) {
        url.replacen(FULL, "/img/avatar_thumbnail/plain/", 1).into()
    } else {
        url.into()
    }
}

#[derive(Default)]
pub struct ImageCache {
    entries: HashMap<String, Entry>,
    bytes: usize,
    clock: Cell<u64>,
    /// Images drawn while missing, to fetch on the next update.
    wanted: RefCell<HashSet<String>>,
    /// When a dropped or failed image was last drawn.
    last_gap: Cell<Option<Instant>>,
    /// Hashes of the URLs of forgotten entries, so they are fetched again when drawn.
    forgotten: HashSet<u64>,
    hasher: RandomState,
    /// Images are fetched only once drawn (the data saver), not when posts arrive.
    lazy: bool,
}

impl ImageCache {
    /// The image's state, marking it as just used. A dropped or retryable image is
    /// queued to be fetched again.
    pub fn get(&self, url: &str) -> Option<&State> {
        let Some(e) = self.entries.get(url) else {
            if self.lazy || self.forgotten.contains(&self.hasher.hash_one(url)) {
                self.wanted.borrow_mut().insert(url.to_string());
            }
            return None;
        };
        let now = self.clock.get() + 1;
        self.clock.set(now);
        e.used.set(now);
        e.drawn.set(Some(Instant::now()));
        if matches!(e.state, State::Evicted)
            || matches!(e.state, State::Failed(_)) && e.tries < MAX_TRIES
        {
            self.last_gap.set(Some(Instant::now()));
        }
        if Self::refetchable(e) {
            self.wanted.borrow_mut().insert(url.to_string());
        }
        Some(&e.state)
    }

    pub fn failed(&self, url: &str) -> bool {
        matches!(
            self.entries.get(url).map(|e| &e.state),
            Some(State::Failed(_))
        )
    }

    /// Marks `url` as loading and returns true if it should be downloaded: it is
    /// new, was dropped, or failed and is due a retry.
    pub fn start(&mut self, url: &str) -> bool {
        match self.entries.get_mut(url) {
            Some(e) if Self::refetchable(e) => {
                self.wanted.borrow_mut().remove(url);
                e.state = State::Loading;
                true
            }
            Some(_) => false,
            None => {
                self.wanted.borrow_mut().remove(url);
                self.forgotten.remove(&self.hasher.hash_one(url));
                self.entries.insert(
                    url.to_string(),
                    Entry {
                        state: State::Loading,
                        size: 0,
                        used: Cell::new(self.clock.get()),
                        drawn: Cell::new(None),
                        tries: 0,
                    },
                );
                true
            }
        }
    }

    /// Stores a finished download: `None` if it failed.
    pub fn finish(&mut self, url: String, state: Option<State>) {
        let Some(e) = self.entries.get_mut(&url) else {
            return;
        };
        self.bytes -= e.size;
        e.size = 0;
        if let Some(state) = state {
            e.size = size_of(&state);
            e.state = state;
            e.tries = 0;
        } else {
            e.tries += 1;
            e.state = State::Failed(Instant::now());
        }
        self.bytes += e.size;
        self.evict();
        self.forget();
    }

    /// Fetch images only once they are drawn.
    pub fn set_lazy(&mut self, on: bool) {
        self.lazy = on;
    }

    /// Images to fetch again, gathered while drawing.
    pub fn take_wanted(&self) -> Vec<String> {
        self.wanted.borrow_mut().drain().collect()
    }

    /// Whether to keep checking for images to fetch without any other event to
    /// prompt it: a dropped or failed image was on screen a moment ago.
    pub fn needs_tick(&self) -> bool {
        !self.wanted.borrow().is_empty()
            || self
                .last_gap
                .get()
                .is_some_and(|t| t.elapsed() < Duration::from_secs(30))
    }

    fn refetchable(e: &Entry) -> bool {
        match e.state {
            State::Evicted => true,
            State::Failed(at) => {
                e.tries < MAX_TRIES && at.elapsed() >= RETRY_AFTER * (1 << (e.tries - 1).min(4))
            }
            _ => false,
        }
    }

    /// Drops the least recently drawn images until usage is under the low-water mark,
    /// or until only the ones on screen are left.
    fn evict(&mut self) {
        if self.bytes <= BUDGET {
            return;
        }
        let mut held: Vec<(u64, String)> = self
            .entries
            .iter()
            .filter(|(_, e)| e.size > 0)
            .filter(|(_, e)| e.drawn.get().is_none_or(|t| t.elapsed() >= ON_SCREEN))
            .map(|(u, e)| (e.used.get(), u.clone()))
            .collect();
        held.sort_unstable();
        for (_, url) in held {
            if self.bytes <= LOW_WATER {
                break;
            }
            if let Some(e) = self.entries.get_mut(&url) {
                self.bytes -= e.size;
                e.size = 0;
                e.state = State::Evicted;
            }
        }
    }

    /// Past [`MAX_ENTRIES`], forgets the least recently drawn entries that hold
    /// nothing, down to three quarters of it.
    fn forget(&mut self) {
        if self.entries.len() <= MAX_ENTRIES {
            return;
        }
        let mut empty: Vec<(u64, String)> = self
            .entries
            .iter()
            .filter(|(_, e)| matches!(e.state, State::Evicted | State::Failed(_)))
            .map(|(u, e)| (e.used.get(), u.clone()))
            .collect();
        empty.sort_unstable();
        let extra = self.entries.len() - MAX_ENTRIES / 4 * 3;
        for (_, url) in empty.into_iter().take(extra) {
            self.entries.remove(&url);
            self.forgotten.insert(self.hasher.hash_one(&url));
        }
    }

    #[cfg(test)]
    fn bytes(&self) -> usize {
        self.bytes
    }
}

fn size_of(state: &State) -> usize {
    match state {
        State::Ready(h) => handle_size(h),
        State::Animated(frames) => frames.0.iter().map(|(h, _)| handle_size(h)).sum(),
        _ => 0,
    }
}

fn handle_size(h: &Handle) -> usize {
    match h {
        Handle::Bytes(_, b) => b.len(),
        Handle::Rgba { pixels, .. } => pixels.len(),
        Handle::Path(..) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[expect(clippy::unnecessary_wraps, reason = "matches what finish() takes")]
    fn ready(n: usize) -> Option<State> {
        Some(State::Ready(Handle::from_bytes(vec![0u8; n])))
    }

    #[test]
    fn lazy_fetches_only_what_is_drawn() {
        let mut c = ImageCache::default();
        assert!(c.get("u").is_none());
        assert!(c.take_wanted().is_empty(), "never asked for, so not wanted");
        c.set_lazy(true);
        assert!(c.get("u").is_none());
        assert_eq!(c.take_wanted(), vec!["u".to_string()]);
    }

    #[test]
    fn evicts_least_recently_drawn_and_refetches() {
        let mut c = ImageCache::default();
        let mb = 1024 * 1024;
        for i in 0..4 {
            let url = format!("u{i}");
            assert!(c.start(&url));
            c.finish(url, ready(50 * mb));
        }
        // 200 MB is over budget, so the oldest go until under 120 MB.
        assert!(c.bytes() <= LOW_WATER);
        assert!(matches!(c.get("u0"), Some(State::Evicted)));
        assert!(matches!(c.get("u3"), Some(State::Ready(_))));
        // Drawing a dropped image asks for it again.
        assert_eq!(c.take_wanted(), vec!["u0".to_string()]);
        assert!(c.start("u0"));
        assert!(!c.start("u3"));
    }

    #[test]
    fn images_on_screen_are_kept_even_over_budget() {
        let mut c = ImageCache::default();
        let mb = 1024 * 1024;
        for i in 0..4 {
            let url = format!("u{i}");
            assert!(c.start(&url));
            // On screen while it loads.
            c.get(&url);
            c.finish(url, ready(50 * mb));
        }
        assert!(c.bytes() > BUDGET);
        assert!((0..4).all(|i| matches!(c.get(&format!("u{i}")), Some(State::Ready(_)))));
    }

    #[test]
    fn empty_entries_are_forgotten_but_fetched_again_when_drawn() {
        let mut c = ImageCache::default();
        for i in 0..=MAX_ENTRIES {
            let url = format!("u{i}");
            assert!(c.start(&url));
            c.finish(url, None);
        }
        assert_eq!(c.entries.len(), MAX_ENTRIES / 4 * 3);
        // The oldest went; drawing one asks for it again.
        assert!(c.get("u0").is_none());
        assert_eq!(c.take_wanted(), vec!["u0".to_string()]);
        assert!(c.start("u0"));
        // A URL never seen is not fetched just because it was drawn.
        assert!(c.get("new").is_none());
        assert!(c.take_wanted().is_empty());
    }

    #[test]
    fn failures_wait_before_retrying() {
        let mut c = ImageCache::default();
        assert!(c.start("x"));
        c.finish("x".into(), None);
        assert!(c.failed("x"));
        assert!(!c.start("x"));
        assert!(!c.needs_tick());
        // Once it is on screen, ticks keep coming until the retry is due.
        c.get("x");
        assert!(c.needs_tick());
        assert!(c.take_wanted().is_empty());
    }
}
