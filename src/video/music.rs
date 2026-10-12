//! Playing a song: an MP3 file, decoded a few seconds ahead on its own thread and
//! played through the same sound output as video.

use super::audio::Buffer;
use super::error::PlayError;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;

/// Seconds of sound decoded ahead of what is heard.
const AHEAD_SECS: usize = 4;

/// A song that is playing. It stops when dropped.
pub struct Music {
    buffer: Arc<Buffer>,
}

impl Music {
    /// Starts playing an MP3 file.
    pub fn play(mp3: Vec<u8>) -> Result<Self, PlayError> {
        Self::play_into(mp3, Arc::new(Buffer::default()))
    }

    fn play_into(mp3: Vec<u8>, buffer: Arc<Buffer>) -> Result<Self, PlayError> {
        let stream = MediaSourceStream::new(
            Box::new(std::io::Cursor::new(mp3)),
            MediaSourceStreamOptions::default(),
        );
        let mut hint = Hint::new();
        hint.with_extension("mp3");
        let mut format = symphonia::default::get_probe()
            .probe(
                &hint,
                stream,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|_| PlayError::Unsupported)?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or(PlayError::Empty)?;
        let id = track.id;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or(PlayError::Unsupported)?;
        let mut decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(|_| PlayError::Unsupported)?;
        let feed = buffer.clone();
        std::thread::Builder::new()
            .name("music".into())
            .spawn(move || {
                let mut samples: Vec<f32> = Vec::new();
                let mut time = 0.0;
                while let Ok(Some(packet)) = format.next_packet() {
                    if feed.stop.load(Ordering::Relaxed) {
                        return;
                    }
                    if packet.track_id != id {
                        continue;
                    }
                    let Ok(audio) = decoder.decode(&packet) else {
                        continue;
                    };
                    let rate = audio.spec().rate();
                    let channels = audio.spec().channels().count() as u16;
                    audio.copy_to_vec_interleaved(&mut samples);
                    feed.push(&samples, time, rate, channels);
                    time += samples.len() as f64 / (f64::from(rate) * f64::from(channels)).max(1.0);
                    let ahead = rate as usize * usize::from(channels) * AHEAD_SECS;
                    while feed.queued() > ahead && !feed.stop.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
                feed.set_ended();
            })
            .map_err(|_| PlayError::NoThread("music player"))?;
        Ok(Self { buffer })
    }

    /// Seconds played so far.
    pub fn time(&self) -> f64 {
        self.buffer.time().unwrap_or(0.0)
    }

    /// The song has played to its end, or there is no sound output to play it on.
    pub fn ended(&self) -> bool {
        self.buffer.done()
    }

    pub fn paused(&self) -> bool {
        self.buffer.paused.load(Ordering::SeqCst)
    }

    pub fn set_paused(&self, paused: bool) {
        self.buffer.paused.store(paused, Ordering::SeqCst);
    }
}

impl Drop for Music {
    fn drop(&mut self) {
        self.buffer.stop.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_not_an_mp3_does_not_play() {
        let err = Music::play_into(b"not a song".to_vec(), Arc::new(Buffer::silent()));
        assert!(matches!(err, Err(PlayError::Unsupported)));
    }

    #[test]
    fn decodes_an_mp3_ahead_of_playing_it() {
        // Half a second of a 440 Hz tone, mono at 22.05 kHz.
        let mp3 = include_bytes!("testdata/tone.mp3").to_vec();
        let buffer = Arc::new(Buffer::silent());
        let m = Music::play_into(mp3, buffer.clone()).unwrap();
        let start = std::time::Instant::now();
        while buffer.queued() < 11_025 && start.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            buffer.queued() >= 11_025,
            "only {} samples",
            buffer.queued()
        );
        assert!(m.time().abs() < f64::EPSILON, "nothing heard yet");
    }

    #[test]
    fn stopping_ends_it() {
        let m = Music {
            buffer: Arc::new(Buffer::silent()),
        };
        assert!(!m.ended());
        m.set_paused(true);
        assert!(m.paused());
        let buffer = m.buffer.clone();
        drop(m);
        assert!(buffer.done());
    }
}
