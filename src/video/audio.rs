//! Sound for the video player: AAC decoding, a buffer the decoder fills, and a
//! rodio source that plays from it and doubles as the playback clock.

use rodio::Source;
use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_AAC, CodecParameters, Decoder, DecoderOptions};
use symphonia::core::formats::Packet;
use symphonia::default::codecs::AacDecoder;

/// Samples handed to the output at a time. Small, so the clock stays close to what is
/// heard.
const CHUNK: usize = 512;

/// The default output's mixer, opened once on a thread that keeps the device open.
/// `None` when there is no sound device.
#[expect(clippy::print_stderr, reason = "no logging module yet")]
fn mixer() -> Option<&'static rodio::mixer::Mixer> {
    static MIXER: OnceLock<Option<rodio::mixer::Mixer>> = OnceLock::new();
    MIXER
        .get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("audio-out".into())
                .spawn(
                    move || match rodio::DeviceSinkBuilder::open_default_sink() {
                        Ok(mut sink) => {
                            sink.log_on_drop(false);
                            let _ = tx.send(Some(sink.mixer().clone()));
                            // The device closes when the sink drops, so keep it for good.
                            loop {
                                std::thread::park();
                            }
                        }
                        Err(e) => {
                            eprintln!("no sound output: {e}");
                            let _ = tx.send(None);
                        }
                    },
                )
                .ok()?;
            rx.recv().ok().flatten()
        })
        .as_ref()
}

#[derive(Default)]
struct Inner {
    samples: VecDeque<f32>,
    /// Playback time of the first sample after the last reset, once one has arrived.
    base: Option<f64>,
    /// Samples played since the last reset.
    consumed: u64,
    rate: u32,
    channels: u16,
    /// The decoder has reached the end; once the samples run out, the clock stops.
    ended: bool,
}

/// Decoded sound waiting to be played, shared by the decoder and the output.
#[derive(Default)]
pub struct Buffer {
    inner: Mutex<Inner>,
    pub paused: AtomicBool,
    pub muted: AtomicBool,
    pub stop: AtomicBool,
    started: AtomicBool,
}

impl Buffer {
    /// A buffer that never opens a sound device, as if one were already playing it.
    #[cfg(test)]
    pub fn silent() -> Self {
        let b = Self::default();
        b.started.store(true, Ordering::SeqCst);
        b
    }

    /// Drops everything queued, as after a seek.
    pub fn reset(&self) {
        let mut i = super::lock(&self.inner);
        i.samples.clear();
        i.base = None;
        i.consumed = 0;
        i.ended = false;
    }

    pub fn set_ended(&self) {
        super::lock(&self.inner).ended = true;
    }

    /// Queues interleaved samples that start at `time` seconds, starting playback
    /// on the first call.
    pub(super) fn push(self: &Arc<Self>, samples: &[f32], time: f64, rate: u32, channels: u16) {
        {
            let mut i = super::lock(&self.inner);
            i.rate = rate;
            i.channels = channels;
            i.base.get_or_insert(time);
            i.samples.extend(samples);
        }
        if !self.started.swap(true, Ordering::SeqCst) {
            match mixer() {
                Some(m) => m.add(Output {
                    buffer: self.clone(),
                    chunk: Vec::with_capacity(CHUNK),
                    pos: 0,
                    rate: NonZero::new(rate).unwrap_or(NonZero::<u32>::MIN),
                    channels: NonZero::new(channels).unwrap_or(NonZero::<u16>::MIN),
                }),
                // With nowhere to play it, the sound can't drive the clock.
                None => self.stop.store(true, Ordering::SeqCst),
            }
        }
    }

    /// Samples waiting to be played.
    pub(super) fn queued(&self) -> usize {
        super::lock(&self.inner).samples.len()
    }

    /// Everything has been played, or there is nowhere to play it.
    pub(super) fn done(&self) -> bool {
        let i = super::lock(&self.inner);
        self.stop.load(Ordering::SeqCst) || (i.ended && i.samples.is_empty())
    }

    /// Where playback is by the sound played so far, or `None` when the sound isn't
    /// driving the clock (no sound yet, none left, or no output).
    pub fn time(&self) -> Option<f64> {
        if self.stop.load(Ordering::SeqCst) || !self.started.load(Ordering::SeqCst) {
            return None;
        }
        let i = super::lock(&self.inner);
        let base = i.base?;
        if i.ended && i.samples.is_empty() {
            return None;
        }
        let per_sec = f64::from(i.rate) * f64::from(i.channels);
        Some(base + i.consumed as f64 / per_sec.max(1.0))
    }
}

/// Plays a [`Buffer`]. Silence while paused, muted or waiting for more; the samples
/// played while muted still count, so the clock keeps going.
struct Output {
    buffer: Arc<Buffer>,
    chunk: Vec<f32>,
    pos: usize,
    rate: rodio::SampleRate,
    channels: rodio::ChannelCount,
}

impl Iterator for Output {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let b = &self.buffer;
        if b.stop.load(Ordering::Relaxed) {
            return None;
        }
        if self.pos >= self.chunk.len() {
            self.chunk.clear();
            self.pos = 0;
            if !b.paused.load(Ordering::Relaxed) {
                let mut i = super::lock(&b.inner);
                let n = CHUNK.min(i.samples.len());
                self.chunk.extend(i.samples.drain(..n));
                i.consumed += n as u64;
            }
            if self.chunk.is_empty() {
                return Some(0.0);
            }
        }
        let s = self.chunk[self.pos];
        self.pos += 1;
        Some(if b.muted.load(Ordering::Relaxed) {
            0.0
        } else {
            s
        })
    }
}

impl Source for Output {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        self.channels
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        self.rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

/// Decodes AAC frames from ADTS and queues them for playing.
pub struct AacStream {
    decoder: Option<(AacDecoder, [u8; 2])>,
    buffer: Arc<Buffer>,
    samples: Option<SampleBuffer<f32>>,
}

impl AacStream {
    pub fn new(buffer: Arc<Buffer>) -> Self {
        Self {
            decoder: None,
            buffer,
            samples: None,
        }
    }

    /// Plays one PES packet of ADTS frames starting at `time`. Sound before
    /// `not_before` is dropped.
    #[expect(clippy::print_stderr, reason = "no logging module yet")]
    pub fn push(&mut self, adts: &[u8], time: f64, not_before: f64) {
        let mut t = time;
        for (h, raw) in super::ts::adts_frames(adts) {
            let rate = h.sample_rate();
            let frame_secs = 1024.0 / f64::from(rate);
            let config = h.config();
            if self.decoder.as_ref().is_none_or(|(_, c)| *c != config) {
                let mut params = CodecParameters::new();
                params
                    .for_codec(CODEC_TYPE_AAC)
                    .with_sample_rate(rate)
                    .with_extra_data(Box::new(config));
                match AacDecoder::try_new(&params, &DecoderOptions::default()) {
                    Ok(d) => self.decoder = Some((d, config)),
                    Err(e) => {
                        eprintln!("can't play this sound: {e}");
                        self.decoder = None;
                        return;
                    }
                }
            }
            let start = t;
            t += frame_secs;
            if t <= not_before {
                continue;
            }
            let Some((dec, _)) = &mut self.decoder else {
                return;
            };
            let Ok(decoded) = dec.decode(&Packet::new_from_slice(0, 0, 1024, raw)) else {
                continue;
            };
            let spec = *decoded.spec();
            let channels = spec.channels.count() as u16;
            let sb = self
                .samples
                .get_or_insert_with(|| SampleBuffer::new(decoded.capacity() as u64, spec));
            if sb.capacity() < decoded.capacity() * usize::from(channels) {
                *sb = SampleBuffer::new(decoded.capacity() as u64, spec);
            }
            sb.copy_interleaved_ref(decoded);
            let mut out = sb.samples();
            let mut at = start;
            if start < not_before {
                // Trim the part of the frame before the seek point.
                let skip =
                    ((not_before - start) * f64::from(rate)) as usize * usize::from(channels);
                out = &out[skip.min(out.len())..];
                at = not_before;
            }
            self.buffer.push(out, at, spec.rate, channels);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A buffer that counts as playing without opening a sound device.
    fn playing() -> Arc<Buffer> {
        Arc::new(Buffer::silent())
    }

    fn output(b: &Arc<Buffer>) -> Output {
        Output {
            buffer: b.clone(),
            chunk: Vec::new(),
            pos: 0,
            rate: NonZero::<u32>::MIN,
            channels: NonZero::<u16>::MIN,
        }
    }

    #[test]
    fn the_sound_played_is_the_clock() {
        let b = playing();
        assert_eq!(b.time(), None, "no sound yet");
        // One second of mono at 1 kHz, starting 5 s in.
        b.push(&[0.5; 1000], 5.0, 1000, 1);
        assert_eq!(b.time(), Some(5.0));
        let mut out = output(&b);
        for _ in 0..CHUNK {
            assert_eq!(out.next(), Some(0.5));
        }
        assert_eq!(b.time(), Some(5.0 + CHUNK as f64 / 1000.0));
        // Once the decoder is done and the samples run out, the clock is let go.
        b.set_ended();
        out.by_ref().take(1000).for_each(drop);
        assert_eq!(b.time(), None);
        b.reset();
        b.push(&[0.0; 10], 1.0, 1000, 1);
        assert_eq!(b.time(), Some(1.0), "a seek starts the clock over");
    }

    #[test]
    fn muted_sound_still_moves_the_clock_but_paused_sound_does_not() {
        let b = playing();
        b.push(&[0.5; 2 * CHUNK], 0.0, 1000, 1);
        let mut out = output(&b);
        b.muted.store(true, Ordering::SeqCst);
        assert_eq!(out.next(), Some(0.0));
        assert!(b.time().unwrap() > 0.0);

        let b = playing();
        b.push(&[0.5; CHUNK], 0.0, 1000, 1);
        let mut out = output(&b);
        b.paused.store(true, Ordering::SeqCst);
        assert_eq!(out.next(), Some(0.0), "silence while paused");
        assert_eq!(b.time(), Some(0.0));
        b.stop.store(true, Ordering::SeqCst);
        assert_eq!(out.next(), None, "stopping ends the source");
        assert_eq!(b.time(), None);
    }

    #[test]
    fn aac_before_the_seek_point_is_dropped() {
        let init = super::super::mp4::init(include_bytes!("mp4/testdata/a_init.mp4"));
        let d = super::super::mp4::demux(include_bytes!("mp4/testdata/a_0.m4s"), &init);
        let adts: Vec<u8> = d.audio.iter().flat_map(|u| u.data.clone()).collect();
        let frames = d.audio.len();

        let b = playing();
        AacStream::new(b.clone()).push(&adts, 0.0, 0.0);
        let all = super::super::lock(&b.inner).samples.len();
        // Stereo 48 kHz: 1024 samples per channel per frame.
        assert_eq!(all, frames * 1024 * 2);

        let b = playing();
        // Halfway through the second frame.
        let seek = 1.5 * 1024.0 / 48_000.0;
        AacStream::new(b.clone()).push(&adts, 0.0, seek);
        let i = super::super::lock(&b.inner);
        assert_eq!(i.base, Some(seek));
        assert_eq!(i.samples.len(), all - 3 * 1024);
    }
}
