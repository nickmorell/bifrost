//! Where sound goes: the speakers or headphones it plays through. One output is
//! open at a time, on the device the person picked or the system's default. Picking
//! another moves whatever is playing to it without stopping.

use super::{Buffer, Output};
use rodio::DeviceTrait;
use rodio::cpal::traits::HostTrait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, Weak};

/// A device sound can play through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Speaker {
    /// Stays the same between runs, so it can be saved.
    pub id: String,
    /// The device's name, for people.
    pub name: String,
}

/// The output devices on this computer, in the system's order. Slow on some
/// systems, so call it off the window's thread.
pub fn speakers() -> Vec<Speaker> {
    let Ok(devices) = rodio::cpal::default_host().output_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|d| {
            Some(Speaker {
                id: d.id().ok()?.to_string(),
                name: d.description().ok()?.name().to_string(),
            })
        })
        .collect()
}

/// Plays through the device with this id from now on, or the system's default with
/// `None` (or when that device isn't there). Anything playing moves over.
pub fn use_speaker(id: Option<String>) {
    let mut out = super::super::lock(&OUT);
    if out.chosen == id {
        return;
    }
    out.chosen = id;
    if out.open.is_some() {
        reopen(&mut out);
    }
}

/// Bumped each time the output changes. A source added for an older one ends, so
/// nothing plays twice.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// The open output's generation.
pub(super) fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

/// Whether a source added at `generation` still belongs to the open output.
pub(super) fn current(generation: u64) -> bool {
    self::generation() == generation
}

struct Out {
    chosen: Option<String>,
    /// The open output's mixer and the way to close it; `None` until something plays.
    open: Option<(rodio::mixer::Mixer, Sender<()>)>,
    /// What has played, to move over when the output changes.
    playing: Vec<Weak<Buffer>>,
}

static OUT: Mutex<Out> = Mutex::new(Out {
    chosen: None,
    open: None,
    playing: Vec::new(),
});

/// Starts playing `buffer`. Returns false when there is no sound device.
pub(super) fn play(buffer: &Arc<Buffer>) -> bool {
    let mut out = super::super::lock(&OUT);
    if out.open.is_none() {
        reopen(&mut out);
    }
    let Some((mixer, _)) = &out.open else {
        return false;
    };
    mixer.add(Output::new(buffer.clone(), generation()));
    out.playing.retain(|b| b.strong_count() > 0);
    out.playing.push(Arc::downgrade(buffer));
    true
}

/// Opens the chosen device, moves what is playing onto it, then closes the old one.
fn reopen(out: &mut Out) {
    let Some(mixer) = open(out.chosen.clone()) else {
        return;
    };
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    out.playing.retain(|b| b.strong_count() > 0);
    for buffer in out.playing.iter().filter_map(Weak::upgrade) {
        if !buffer.stop.load(Ordering::Relaxed) {
            mixer.0.add(Output::new(buffer, generation));
        }
    }
    // Dropping the old sender closes the old device.
    out.open = Some(mixer);
}

/// Opens a device on a thread of its own, which keeps it open until told to stop.
/// The chosen device if it is there, otherwise the default.
#[expect(clippy::print_stderr, reason = "no logging module yet")]
fn open(chosen: Option<String>) -> Option<(rodio::mixer::Mixer, Sender<()>)> {
    let (tx, rx) = channel();
    let (close, closed) = channel::<()>();
    std::thread::Builder::new()
        .name("audio-out".into())
        .spawn(move || {
            let device = chosen
                .and_then(|id| id.parse().ok())
                .and_then(|id| rodio::cpal::default_host().device_by_id(&id));
            let sink = match device {
                Some(d) => rodio::DeviceSinkBuilder::from_device(d)
                    .and_then(rodio::DeviceSinkBuilder::open_stream)
                    .or_else(|_| rodio::DeviceSinkBuilder::open_default_sink()),
                None => rodio::DeviceSinkBuilder::open_default_sink(),
            };
            match sink {
                Ok(mut sink) => {
                    sink.log_on_drop(false);
                    let _ = tx.send(Some(sink.mixer().clone()));
                    // Until the output changes or BiFrost closes.
                    let _ = closed.recv();
                }
                Err(e) => {
                    eprintln!("no sound output: {e}");
                    let _ = tx.send(None);
                }
            }
        })
        .ok()?;
    Some((rx.recv().ok().flatten()?, close))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_added_to_an_earlier_output_stops_there() {
        let b = Arc::new(Buffer::silent());
        b.push(&[0.5; 8], 0.0, 1000, 1);
        let mut now = Output::new(b.clone(), generation());
        assert_eq!(now.next(), Some(0.5));
        let mut before = Output::new(b, generation().wrapping_sub(1));
        assert_eq!(before.next(), None, "it moved to the new output");
    }

    #[test]
    fn choosing_the_same_device_again_changes_nothing() {
        let before = generation();
        use_speaker(None);
        assert_eq!(generation(), before);
    }
}
