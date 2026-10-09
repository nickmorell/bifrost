//! The frames on screen. Whatever draws the video moves it on each time it draws,
//! so a playing video redraws only its own picture, at its own frame rate.

use super::{Shared, Status, lock};
use iced::widget::image;
use std::sync::Arc;
use std::time::Duration;

/// The frame on screen and the one before it, drawn underneath so that a frame the
/// renderer hasn't taken up yet doesn't flash black.
#[derive(Default)]
pub(super) struct Screen {
    current: Option<image::Handle>,
    previous: Option<image::Handle>,
}

/// A playing video's pictures, shared with whatever draws them.
#[derive(Clone)]
pub struct Pictures(pub(super) Arc<Shared>);

impl std::fmt::Debug for Pictures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pictures").finish_non_exhaustive()
    }
}

/// How soon to look again while waiting for frames, and the longest a playing
/// video waits between looks.
const LOADING_STEP: Duration = Duration::from_millis(100);
const MAX_STEP: Duration = Duration::from_millis(100);

impl Pictures {
    /// Moves on to the frame due now. Returns how long until the next one is due, or
    /// `None` when nothing changes without input (paused, ended or failed).
    pub fn advance(&self) -> Option<Duration> {
        let sh = &self.0;
        let mut st = lock(&sh.state);
        let now = sh.time(&mut st);
        let mut next = None;
        while st.frames.front().is_some_and(|f| f.time <= now) {
            next = st.frames.pop_front().map(|f| f.handle);
        }
        if next.is_some() {
            let mut screen = lock(&sh.screen);
            screen.previous = std::mem::replace(&mut screen.current, next);
        }
        if st.status == Status::Playing
            && st.eof
            && st.frames.is_empty()
            && sh.audio.time().is_none()
        {
            st.status = Status::Ended;
            let end = now.min(st.duration.max(now));
            st.clock.set(end, false);
        }
        let paused = sh.paused.load(std::sync::atomic::Ordering::SeqCst);
        match st.status {
            Status::Loading => Some(LOADING_STEP),
            Status::Playing if !paused => Some(st.frames.front().map_or(MAX_STEP, |f| {
                Duration::from_secs_f64((f.time - now).clamp(0.0, MAX_STEP.as_secs_f64()))
            })),
            _ => None,
        }
    }

    /// The frame on screen and the one before it.
    pub fn frame(&self) -> Option<(image::Handle, Option<image::Handle>)> {
        let screen = lock(&self.0.screen);
        Some((screen.current.clone()?, screen.previous.clone()))
    }
}
