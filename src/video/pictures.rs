//! The frames on screen. Whatever draws the video moves it on each time it draws,
//! so a playing video redraws only its own picture, at its own frame rate.

use super::{Shared, Status, lock};
use iced::advanced::image::Allocation;
use iced::widget::image;
use std::sync::Arc;
use std::time::Duration;

/// The frame due now and the newest frame the renderer has ready to draw. A frame is
/// drawn only once it is ready: the renderer uploads large images in the background,
/// and drawing one before then shows nothing for that frame, which flickers.
#[derive(Default)]
pub(super) struct Screen {
    due: Option<image::Handle>,
    /// Held so the renderer keeps the frame until a newer one is ready.
    ready: Option<Allocation>,
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
            lock(&sh.screen).due = next;
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

    /// The frame due now, which may not be ready to draw yet.
    pub fn due(&self) -> Option<image::Handle> {
        lock(&self.0.screen).due.clone()
    }

    /// The newest frame ready to draw.
    pub fn ready(&self) -> Option<image::Handle> {
        lock(&self.0.screen)
            .ready
            .as_ref()
            .map(|r| r.handle().clone())
    }

    /// Draws `frame` from now on: the renderer has it ready.
    pub fn uploaded(&self, frame: Allocation) {
        lock(&self.0.screen).ready = Some(frame);
    }
}
