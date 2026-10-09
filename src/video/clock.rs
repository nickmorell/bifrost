//! The wall clock, for video with no sound to follow.

use std::time::Instant;

/// The wall clock, used when there is no sound to follow.
#[derive(Default)]
pub(super) struct Clock {
    base: f64,
    since: Option<Instant>,
}

impl Clock {
    pub(super) fn now(&self) -> f64 {
        self.base + self.since.map_or(0.0, |s| s.elapsed().as_secs_f64())
    }

    pub(super) fn set(&mut self, t: f64, running: bool) {
        self.base = t;
        self.since = running.then(Instant::now);
    }
}
