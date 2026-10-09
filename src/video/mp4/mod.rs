//! Pulling H.264 video and AAC audio out of fragmented MP4 (CMAF) segments, the other
//! format HLS uses: an init segment describes the tracks once, and each media segment
//! holds `moof` (where the samples are and when they play) and `mdat` (the samples).
//! Out come the same units as from a transport stream, so the decoder treats both
//! alike: video as Annex B access units, audio as ADTS frames, times in 90 kHz ticks.

mod boxes;
mod init;

use super::ts::{Demuxed, Unit};
use boxes::{box_len, boxes, child, children, u32_at, u64_at};
use init::Kind;
pub use init::{Init, read as init};

/// Whether `data` starts like an MP4 file or segment rather than a transport stream.
pub fn looks_like(data: &[u8]) -> bool {
    data.get(4..8).is_some_and(|t| {
        matches!(
            t,
            b"ftyp" | b"styp" | b"moof" | b"moov" | b"sidx" | b"emsg" | b"prft" | b"free" | b"uuid"
        )
    })
}

/// Splits a media segment's fragments into video and audio units.
pub fn demux(data: &[u8], init: &Init) -> Demuxed {
    let mut out = Demuxed::default();
    let mut started = Vec::new();
    let mut offset = 0;
    for (kind, body) in boxes(data) {
        let start = offset;
        offset += box_len(&data[start..]);
        if kind != *b"moof" {
            continue;
        }
        for traf in children(body, b"traf") {
            fragment(data, start, traf, init, &mut out, &mut started);
        }
    }
    out
}

/// One track's samples in one fragment.
fn fragment(
    file: &[u8],
    moof_start: usize,
    traf: &[u8],
    init: &Init,
    out: &mut Demuxed,
    started: &mut Vec<u32>,
) {
    let Some(tfhd) = child(traf, *b"tfhd") else {
        return;
    };
    let (Some(flags), Some(id)) = (u32_at(tfhd, 0), u32_at(tfhd, 4)) else {
        return;
    };
    let Some(track) = init.tracks.iter().find(|t| t.id == id) else {
        return;
    };
    let mut at = 8;
    let mut base = moof_start;
    if flags & 0x01 != 0 {
        base = u64_at(tfhd, at)
            .and_then(|b| usize::try_from(b).ok())
            .unwrap_or(base);
        at += 8;
    }
    if flags & 0x02 != 0 {
        at += 4;
    }
    let mut duration = track.defaults.duration;
    let mut size = track.defaults.size;
    if flags & 0x08 != 0 {
        duration = u32_at(tfhd, at).unwrap_or(duration);
        at += 4;
    }
    if flags & 0x10 != 0 {
        size = u32_at(tfhd, at).unwrap_or(size);
    }
    let mut time = child(traf, *b"tfdt").map_or(0, |t| {
        if t.first() == Some(&1) {
            u64_at(t, 4).unwrap_or(0)
        } else {
            u32_at(t, 4).map_or(0, u64::from)
        }
    });
    let mut cursor = base;
    for trun in children(traf, b"trun") {
        let Some(samples) = run(trun, duration, size) else {
            return;
        };
        if let Some(offset) = samples.data_offset {
            cursor = base.saturating_add_signed(offset as isize);
        }
        for s in samples.items {
            let Some(end) = cursor.checked_add(s.size as usize) else {
                return;
            };
            let Some(bytes) = file.get(cursor..end) else {
                return;
            };
            cursor = end;
            let pts = time.saturating_add_signed(i64::from(s.offset));
            time = time.saturating_add(u64::from(s.duration));
            let ticks = u128::from(pts) * 90_000 / u128::from(track.timescale.max(1));
            let ticks = u64::try_from(ticks).unwrap_or(u64::MAX);
            match &track.kind {
                Kind::Video {
                    params,
                    length_size,
                } => {
                    let mut unit = Vec::with_capacity(bytes.len() + params.len() + 16);
                    // The parameter sets go before the track's first picture.
                    if !started.contains(&id) {
                        started.push(id);
                        unit.extend_from_slice(params);
                    }
                    annex_b(bytes, *length_size, &mut unit);
                    out.video.push(Unit {
                        pts: Some(ticks),
                        data: unit,
                    });
                }
                Kind::Audio {
                    object_type,
                    rate_index,
                    channels,
                } => out.audio.push(Unit {
                    pts: Some(ticks),
                    data: adts(*object_type, *rate_index, *channels, bytes),
                }),
                Kind::Other { video: true } => out.unsupported_video = true,
                Kind::Other { video: false } => {}
            }
        }
    }
}

struct Sample {
    duration: u32,
    size: u32,
    /// Presentation time minus decode time, in the track's ticks.
    offset: i32,
}

struct Run {
    data_offset: Option<i32>,
    items: Vec<Sample>,
}

/// A `trun` box: the samples of one run, each with what differs from the defaults.
fn run(trun: &[u8], duration: u32, size: u32) -> Option<Run> {
    let version = *trun.first()?;
    let flags = u32_at(trun, 0)? & 0x00ff_ffff;
    let count = u32_at(trun, 4)?;
    let mut at = 8;
    let mut data_offset = None;
    if flags & 0x001 != 0 {
        data_offset = Some(u32_at(trun, at)? as i32);
        at += 4;
    }
    if flags & 0x004 != 0 {
        at += 4;
    }
    let count = sample_count(flags, count, trun.len().saturating_sub(at))?;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let mut s = Sample {
            duration,
            size,
            offset: 0,
        };
        if flags & 0x100 != 0 {
            s.duration = u32_at(trun, at)?;
            at += 4;
        }
        if flags & 0x200 != 0 {
            s.size = u32_at(trun, at)?;
            at += 4;
        }
        if flags & 0x400 != 0 {
            at += 4;
        }
        if flags & 0x800 != 0 {
            let raw = u32_at(trun, at)?;
            s.offset = if version == 0 {
                i32::try_from(raw).unwrap_or(i32::MAX)
            } else {
                raw as i32
            };
            at += 4;
        }
        items.push(s);
    }
    Some(Run { data_offset, items })
}

/// The most samples a run without per-sample fields may claim. A few seconds of
/// video or audio is a few hundred; this only stops a bad file asking for billions.
const MAX_SAMPLES: usize = 100_000;

/// How many samples a run really holds: no more than its per-sample fields have room
/// for in the `left` bytes after its header, or than [`MAX_SAMPLES`] when it has none.
/// `None` when the run claims more than that.
fn sample_count(flags: u32, count: u32, left: usize) -> Option<usize> {
    let fields = [0x100, 0x200, 0x400, 0x800]
        .iter()
        .filter(|f| flags & *f != 0)
        .count();
    let most = match fields {
        0 => MAX_SAMPLES,
        n => left / (4 * n),
    };
    usize::try_from(count).ok().filter(|c| *c <= most)
}

/// Turns length-prefixed NAL units into start-code-prefixed ones.
fn annex_b(sample: &[u8], length_size: usize, out: &mut Vec<u8>) {
    let mut at = 0;
    while at + length_size <= sample.len() {
        let len = sample[at..at + length_size]
            .iter()
            .fold(0usize, |n, b| (n << 8) | usize::from(*b));
        at += length_size;
        let Some(nal) = sample.get(at..at + len) else {
            return;
        };
        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(nal);
        at += len;
    }
}

/// A raw AAC frame with the 7-byte ADTS header the sound decoder reads.
fn adts(object_type: u8, rate_index: u8, channels: u8, frame: &[u8]) -> Vec<u8> {
    let len = frame.len() + 7;
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(&[
        0xff,
        0xf1,
        ((object_type - 1) << 6) | (rate_index << 2) | (channels >> 2),
        ((channels & 3) << 6) | ((len >> 11) as u8 & 3),
        (len >> 3) as u8,
        ((len as u8 & 7) << 5) | 0x1f,
        0xfc,
    ]);
    out.extend_from_slice(frame);
    out
}

#[cfg(test)]
mod tests;
