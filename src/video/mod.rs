//! The built-in video player. Bluesky serves video as HLS: playlists of short MPEG-TS
//! segments carrying H.264 and AAC. A download thread fetches segments ahead, a decode
//! thread turns them into frames (OpenH264) and sound (Symphonia, played through
//! rodio), and the picture on screen asks [`Pictures`] for the frame that is due each
//! time it is drawn. The sound sets the pace when there is any; otherwise the wall
//! clock does.

mod audio;
mod clock;
mod error;
pub mod hls;
mod mp4;
pub mod music;
mod net;
mod pictures;
mod source;
pub mod ts;

use clock::Clock;
use error::PlayError;
use iced::widget::image;
use net::http;
pub use pictures::Pictures;
use source::Fetched;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, sync_channel};
use std::{cmp::Reverse, collections::BinaryHeap, collections::VecDeque};
use std::{sync::Arc, sync::Mutex, time::Duration};
use ts::time_of;

/// Tallest quality played inline; sharper ones cost far more to decode.
const MAX_HEIGHT: u32 = 720;
/// Most decoded frame data held ahead of playback.
const FRAME_BYTES: usize = 64 * 1024 * 1024;
/// Segments downloaded ahead of the decoder.
const PREFETCH: usize = 2;

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Loading,
    Playing,
    Ended,
    Failed(String),
}

struct Frame {
    time: f64,
    handle: image::Handle,
}

struct State {
    status: Status,
    /// A live stream: no end and no seeking; `duration` is how much has come so far.
    live: bool,
    /// Start time of each segment, in seconds.
    starts: Vec<f64>,
    duration: f64,
    frames: VecDeque<Frame>,
    /// How many frames may wait at once, set from the picture size.
    frame_cap: usize,
    /// The decoder has used up the stream.
    eof: bool,
    /// The segment and time to play from after a seek.
    from: (usize, f64),
    clock: Clock,
    size: Option<(u32, u32)>,
}

struct Shared {
    stop: AtomicBool,
    paused: AtomicBool,
    /// Bumped on each seek; work for an older value is thrown away.
    generation: AtomicU64,
    state: Mutex<State>,
    audio: Arc<audio::Buffer>,
    /// The frame due now and the newest one ready to draw.
    screen: Mutex<pictures::Screen>,
}

impl Shared {
    fn new(audio: Arc<audio::Buffer>) -> Self {
        Self {
            stop: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            state: Mutex::new(State {
                status: Status::Loading,
                live: false,
                starts: Vec::new(),
                duration: 0.0,
                frames: VecDeque::new(),
                frame_cap: 8,
                eof: false,
                from: (0, 0.0),
                clock: Clock::default(),
                size: None,
            }),
            audio,
            screen: Mutex::default(),
        }
    }

    fn stale(&self, generation: u64) -> bool {
        self.stop.load(Ordering::SeqCst) || self.generation.load(Ordering::SeqCst) != generation
    }

    fn fail(&self, why: &PlayError) {
        lock(&self.state).status = Status::Failed(why.to_string());
    }

    /// The playback position. Follows the sound while it plays, keeping the wall
    /// clock in step so it can take over when the sound runs out.
    fn time(&self, st: &mut State) -> f64 {
        let running = st.status == Status::Playing && !self.paused.load(Ordering::SeqCst);
        match self.audio.time() {
            Some(t) => {
                st.clock.set(t, running);
                t
            }
            None => st.clock.now(),
        }
    }
}

/// A playing video. Dropping it stops playback.
pub struct Player {
    shared: Arc<Shared>,
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player").finish_non_exhaustive()
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        self.shared.audio.stop.store(true, Ordering::SeqCst);
    }
}

impl Player {
    /// Starts fetching and playing the HLS playlist at `url`.
    pub fn open(url: &str, muted: bool) -> Self {
        let audio = Arc::new(audio::Buffer::default());
        audio.muted.store(muted, Ordering::SeqCst);
        let shared = Arc::new(Shared::new(audio));
        let s = shared.clone();
        let url = url.to_string();
        let spawned = std::thread::Builder::new()
            .name("video".into())
            .spawn(move || decode_thread(s, url));
        if spawned.is_err() {
            shared.fail(&PlayError::NoThread("player"));
        }
        Self { shared }
    }

    /// The pictures, for the screen to draw and move on as it goes.
    pub fn pictures(&self) -> Pictures {
        Pictures(self.shared.clone())
    }

    pub fn status(&self) -> Status {
        lock(&self.shared.state).status.clone()
    }

    pub fn paused(&self) -> bool {
        self.shared.paused.load(Ordering::SeqCst)
    }

    pub fn muted(&self) -> bool {
        self.shared.audio.muted.load(Ordering::SeqCst)
    }

    /// The picture's size, once the first frame is decoded.
    pub fn size(&self) -> Option<(u32, u32)> {
        lock(&self.shared.state).size
    }

    /// (position, length) in seconds.
    pub fn progress(&self) -> (f64, f64) {
        let mut st = lock(&self.shared.state);
        let t = self.shared.time(&mut st);
        (t.clamp(0.0, st.duration.max(0.0)), st.duration)
    }

    /// Whether playback can move on without input, so the UI should keep ticking.
    pub fn active(&self) -> bool {
        let st = lock(&self.shared.state);
        match st.status {
            Status::Loading => true,
            Status::Playing => !self.paused(),
            _ => false,
        }
    }

    pub fn set_paused(&self, paused: bool) {
        let sh = &self.shared;
        let mut st = lock(&sh.state);
        let t = sh.time(&mut st);
        sh.paused.store(paused, Ordering::SeqCst);
        sh.audio.paused.store(paused, Ordering::SeqCst);
        let running = !paused && st.status == Status::Playing;
        st.clock.set(t, running);
    }

    pub fn set_muted(&self, muted: bool) {
        self.shared.audio.muted.store(muted, Ordering::SeqCst);
    }

    /// A live stream, which can't be sought and has no length.
    pub fn live(&self) -> bool {
        lock(&self.shared.state).live
    }

    /// Plays from `time` seconds, or from the start after the end.
    pub fn seek(&self, time: f64) {
        let sh = &self.shared;
        let mut st = lock(&sh.state);
        if st.live || st.starts.is_empty() || matches!(st.status, Status::Failed(_)) {
            return;
        }
        let time = time.clamp(0.0, (st.duration - 0.1).max(0.0));
        let segment = st.starts.partition_point(|s| *s <= time).saturating_sub(1);
        st.from = (segment, time);
        st.frames.clear();
        st.eof = false;
        st.status = Status::Loading;
        st.clock.set(time, false);
        sh.audio.reset();
        sh.generation.fetch_add(1, Ordering::SeqCst);
    }
}

fn decode_thread(shared: Arc<Shared>, url: String) {
    let plan = match source::runtime() {
        Some(rt) => rt.block_on(source::load(http(), &url)),
        None => Err(PlayError::NoThread("player")),
    };
    let plan = match plan {
        Ok(p) => p,
        Err(e) => return shared.fail(&e),
    };
    {
        let mut st = lock(&shared.state);
        st.live = plan.live();
        if !st.live {
            (st.starts, st.duration) = plan.starts();
        }
    }
    let inits = (
        plan.inits.0.as_deref().map(mp4::init),
        plan.inits.1.as_deref().map(mp4::init),
    );
    let (tx, rx) = sync_channel(PREFETCH);
    let s = shared.clone();
    if std::thread::Builder::new()
        .name("video-download".into())
        .spawn(move || source::download(&s, plan, &tx))
        .is_err()
    {
        return shared.fail(&PlayError::NoThread("download"));
    }
    if let Err(e) = Decoder::new(shared.clone(), inits).and_then(|mut d| d.run(rx)) {
        shared.fail(&e);
    }
}

struct Decoder {
    shared: Arc<Shared>,
    /// The tracks of fragmented MP4 video and sound segments.
    inits: (Option<mp4::Init>, Option<mp4::Init>),
    video: openh264::decoder::Decoder,
    sound: audio::AacStream,
    /// Presentation times of the pictures sent to the decoder and not yet out of it.
    /// Pictures come out in presentation order, so each takes the smallest.
    pending: BinaryHeap<Reverse<u64>>,
    /// Maps the current segment's ticks to seconds: (segment start, its first tick).
    timing: (f64, u64),
    /// Frames before this time are skipped, after a seek.
    not_before: f64,
    /// Time of the last picture queued.
    last_shown: f64,
    generation: u64,
}

fn new_h264() -> Result<openh264::decoder::Decoder, PlayError> {
    use openh264::decoder::{DecoderConfig, Flush};
    openh264::decoder::Decoder::with_api_config(
        openh264::OpenH264API::from_source(),
        DecoderConfig::new().flush_after_decode(Flush::NoFlush),
    )
    .map_err(|e| PlayError::Decode(e.to_string()))
}

impl Decoder {
    fn new(
        shared: Arc<Shared>,
        inits: (Option<mp4::Init>, Option<mp4::Init>),
    ) -> Result<Self, PlayError> {
        let sound = audio::AacStream::new(shared.audio.clone());
        Ok(Self {
            shared,
            inits,
            video: new_h264()?,
            sound,
            pending: BinaryHeap::new(),
            timing: (0.0, 0),
            not_before: 0.0,
            last_shown: f64::MIN,
            generation: u64::MAX,
        })
    }

    fn run(&mut self, rx: Receiver<Fetched>) -> Result<(), PlayError> {
        loop {
            if self.shared.stop.load(Ordering::SeqCst) {
                return Ok(());
            }
            let item = match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(item) => item,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            };
            let current = self.shared.generation.load(Ordering::SeqCst);
            match item {
                Fetched::Failed(e) => return Err(e),
                Fetched::End { generation } if generation == current => {
                    self.flush();
                    self.shared.audio.set_ended();
                    let mut st = lock(&self.shared.state);
                    st.eof = true;
                    // A seek past the last picture has nothing to show; let it end.
                    if st.status == Status::Loading && st.frames.is_empty() {
                        st.status = Status::Playing;
                    }
                }
                Fetched::Segment {
                    generation,
                    start,
                    video,
                    audio,
                } if generation == current => {
                    if generation != self.generation {
                        self.restart(generation)?;
                    }
                    self.segment(start, &video, audio.as_deref())?;
                }
                _ => {}
            }
        }
    }

    /// Starts over after a seek: segments begin with a key frame, so a fresh decoder
    /// picks up cleanly.
    fn restart(&mut self, generation: u64) -> Result<(), PlayError> {
        self.generation = generation;
        self.video = new_h264()?;
        self.pending.clear();
        self.not_before = lock(&self.shared.state).from.1;
        self.last_shown = f64::MIN;
        Ok(())
    }

    fn segment(&mut self, start: f64, data: &[u8], audio: Option<&[u8]>) -> Result<(), PlayError> {
        let demuxed = demux(data, self.inits.0.as_ref());
        if demuxed.unsupported_video || demuxed.video.is_empty() {
            return Err(PlayError::Unsupported);
        }
        {
            let mut st = lock(&self.shared.state);
            if st.live {
                st.duration = st.duration.max(start);
            }
        }
        let first = demuxed
            .video
            .iter()
            .chain(&demuxed.audio)
            .filter_map(|u| u.pts)
            .min()
            .unwrap_or(0);
        self.timing = (start, first);

        // Sound first: it is small, and the clock needs it before the first picture.
        match audio {
            Some(a) if a.first() == Some(&ts::SYNC) || mp4::looks_like(a) => {
                let d = demux(a, self.inits.1.as_ref());
                let first = d.audio.iter().filter_map(|u| u.pts).min().unwrap_or(0);
                let timing = std::mem::replace(&mut self.timing, (start, first));
                for unit in &d.audio {
                    let t = unit.pts.map_or(start, |p| time_of(self.timing, p));
                    self.sound.push(&unit.data, t, self.not_before);
                }
                self.timing = timing;
            }
            Some(a) => self.sound.push(ts::skip_id3(a), start, self.not_before),
            None => {
                for unit in &demuxed.audio {
                    let t = unit.pts.map_or(start, |p| time_of(self.timing, p));
                    self.sound.push(&unit.data, t, self.not_before);
                }
            }
        }

        let mut last = first;
        for unit in &demuxed.video {
            if self.shared.stale(self.generation) {
                return Ok(());
            }
            let pts = unit.pts.unwrap_or(last + 3000);
            last = pts;
            self.pending.push(Reverse(pts));
            // A damaged picture is skipped.
            if let Ok(Some(yuv)) = self.video.decode(&unit.data)
                && let Some(Reverse(pts)) = self.pending.pop()
            {
                let time = time_of(self.timing, pts);
                if wanted(&self.shared, self.not_before, self.last_shown, time) {
                    let picture = to_rgba(&yuv);
                    self.emit(time, picture);
                }
            }
        }
        Ok(())
    }

    fn flush(&mut self) {
        let mut pictures = Vec::new();
        if let Ok(frames) = self.video.flush_remaining() {
            for yuv in &frames {
                let Some(Reverse(pts)) = self.pending.pop() else {
                    break;
                };
                let time = time_of(self.timing, pts);
                if wanted(&self.shared, self.not_before, self.last_shown, time) {
                    pictures.push((time, to_rgba(yuv)));
                }
            }
        }
        for (time, p) in pictures {
            self.emit(time, p);
        }
    }

    /// Queues a decoded picture, waiting while the queue is full.
    fn emit(&mut self, time: f64, (w, h, rgba): (u32, u32, Vec<u8>)) {
        loop {
            let mut st = lock(&self.shared.state);
            // Checked under the lock, since seeking holds it too.
            if self.shared.stale(self.generation) {
                return;
            }
            if st.size.is_none() {
                st.frame_cap = (FRAME_BYTES / rgba.len().max(1)).clamp(6, 90);
            }
            st.size = Some((w, h));
            if st.frames.len() < st.frame_cap {
                self.last_shown = time;
                st.frames.push_back(Frame {
                    time,
                    handle: image::Handle::from_rgba(w, h, rgba),
                });
                if st.status == Status::Loading {
                    st.status = Status::Playing;
                    let running = !self.shared.paused.load(Ordering::SeqCst);
                    let t = self.not_before.max(0.0).min(time);
                    st.clock.set(t, running);
                }
                return;
            }
            drop(st);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

/// A segment's video and sound, from a transport stream or fragmented MP4.
fn demux(data: &[u8], init: Option<&mp4::Init>) -> ts::Demuxed {
    match init {
        Some(init) if mp4::looks_like(data) => mp4::demux(data, init),
        _ => ts::demux(data),
    }
}

/// Whether a picture is worth converting and queueing: not before a seek point, and
/// not already too late to show, as when decoding can't keep up.
/// A late picture still gets through every so often, so slow decoding stutters
/// rather than freezing.
fn wanted(shared: &Shared, not_before: f64, last_queued: f64, time: f64) -> bool {
    if time + 0.001 < not_before {
        return false;
    }
    let mut st = lock(&shared.state);
    st.status != Status::Playing || time - last_queued >= 0.25 || time >= shared.time(&mut st) - 0.1
}

fn to_rgba(yuv: &openh264::decoder::DecodedYUV<'_>) -> (u32, u32, Vec<u8>) {
    use openh264::formats::YUVSource;
    let (w, h) = yuv.dimensions();
    let mut rgba = vec![0; w * h * 4];
    yuv.write_rgba8(&mut rgba);
    (w as u32, h as u32, rgba)
}

#[cfg(test)]
mod tests;

/// Locks a mutex, carrying on with the data if another thread panicked while holding it.
/// The player's state stays valid between steps, so a poisoned lock is still usable.
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
