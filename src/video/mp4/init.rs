//! The init segment: which tracks there are, and how to read each one's samples.

use super::boxes::{boxes, child, children, u16_at, u32_at};

/// What the init segment says about each track.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Init {
    pub(super) tracks: Vec<Track>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Track {
    pub(super) id: u32,
    /// Ticks per second of the track's times.
    pub(super) timescale: u32,
    pub(super) kind: Kind,
    /// Sample duration, size and flags used when a fragment doesn't give them.
    pub(super) defaults: Defaults,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Kind {
    Video {
        /// The sequence and picture parameter sets, in Annex B form.
        params: Vec<u8>,
        /// Bytes in each NAL unit's length prefix.
        length_size: usize,
    },
    Audio {
        /// MPEG-4 audio object type, sample rate index and channel layout.
        object_type: u8,
        rate_index: u8,
        channels: u8,
    },
    /// HEVC, Opus and anything else the player can't decode.
    Other { video: bool },
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Defaults {
    pub(super) duration: u32,
    pub(super) size: u32,
}

/// Reads the tracks from an init segment (`ftyp` and `moov`).
pub fn read(data: &[u8]) -> Init {
    let mut out = Init::default();
    let Some(moov) = child(data, *b"moov") else {
        return out;
    };
    let mut defaults = Vec::new();
    if let Some(mvex) = child(moov, *b"mvex") {
        for trex in children(mvex, b"trex") {
            // version/flags, track id, description index, duration, size, flags.
            if let (Some(id), Some(duration), Some(size)) =
                (u32_at(trex, 4), u32_at(trex, 12), u32_at(trex, 16))
            {
                defaults.push((id, Defaults { duration, size }));
            }
        }
    }
    for trak in children(moov, b"trak") {
        if let Some(mut track) = read_track(trak) {
            if let Some((_, d)) = defaults.iter().find(|(id, _)| *id == track.id) {
                track.defaults = *d;
            }
            out.tracks.push(track);
        }
    }
    out
}

fn read_track(trak: &[u8]) -> Option<Track> {
    let tkhd = child(trak, *b"tkhd")?;
    let id = if tkhd.first()? == &1 {
        u32_at(tkhd, 20)?
    } else {
        u32_at(tkhd, 12)?
    };
    let mdia = child(trak, *b"mdia")?;
    let mdhd = child(mdia, *b"mdhd")?;
    let timescale = if mdhd.first()? == &1 {
        u32_at(mdhd, 20)?
    } else {
        u32_at(mdhd, 12)?
    };
    let handler = child(mdia, *b"hdlr")?.get(8..12)?;
    let stsd = child(child(child(mdia, *b"minf")?, *b"stbl")?, *b"stsd")?;
    // version/flags and entry count, then the first sample entry.
    let (entry_type, entry) = boxes(stsd.get(8..)?).next()?;
    let kind = match (handler, &entry_type) {
        (b"vide", b"avc1" | b"avc3") => avc(entry).unwrap_or(Kind::Other { video: true }),
        (b"vide", _) => Kind::Other { video: true },
        (b"soun", b"mp4a") => aac(entry).unwrap_or(Kind::Other { video: false }),
        (b"soun", _) => Kind::Other { video: false },
        _ => return None,
    };
    Some(Track {
        id,
        timescale: timescale.max(1),
        kind,
        defaults: Defaults::default(),
    })
}

/// An `avc1` sample entry's parameter sets, from its `avcC` box.
fn avc(entry: &[u8]) -> Option<Kind> {
    // The visual sample entry's fixed fields take 78 bytes before its child boxes.
    let c = child(entry.get(78..)?, *b"avcC")?;
    let length_size = usize::from(c.get(4)? & 3) + 1;
    let mut params = Vec::new();
    let mut at = 5;
    for _ in 0..2 {
        let count = if params.is_empty() {
            usize::from(c.get(at)? & 0x1f)
        } else {
            usize::from(*c.get(at)?)
        };
        at += 1;
        for _ in 0..count {
            let len = usize::from(u16_at(c, at)?);
            params.extend_from_slice(&[0, 0, 0, 1]);
            params.extend_from_slice(c.get(at + 2..at + 2 + len)?);
            at += 2 + len;
        }
        if params.is_empty() {
            return None;
        }
    }
    Some(Kind::Video {
        params,
        length_size,
    })
}

/// An `mp4a` sample entry's AAC settings, from the AudioSpecificConfig in its `esds`.
fn aac(entry: &[u8]) -> Option<Kind> {
    // The audio sample entry's fixed fields take 28 bytes before its child boxes.
    let esds = child(entry.get(28..)?, *b"esds")?;
    let asc = descriptor(esds.get(4..)?, 0x03)
        .and_then(|es| {
            // ES id, then flags saying which optional fields follow.
            let flags = *es.get(2)?;
            let mut at = 3;
            if flags & 0x80 != 0 {
                at += 2;
            }
            if flags & 0x40 != 0 {
                at += 1 + usize::from(*es.get(at)?);
            }
            if flags & 0x20 != 0 {
                at += 2;
            }
            descriptor(es.get(at..)?, 0x04)
        })
        // Object type, stream type, buffer size and bit rates come first.
        .and_then(|dc| descriptor(dc.get(13..)?, 0x05))?;
    let object_type = asc.first()? >> 3;
    let rate_index = ((asc.first()? & 0x07) << 1) | (asc.get(1)? >> 7);
    let channels = (asc.get(1)? >> 3) & 0x0f;
    // An escaped object type or an explicit sample rate doesn't fit an ADTS header.
    if object_type == 0 || object_type > 4 || rate_index > 12 {
        return Some(Kind::Other { video: false });
    }
    Some(Kind::Audio {
        object_type,
        rate_index,
        channels,
    })
}

/// The body of the first descriptor with this tag (MPEG-4 Systems' tag, length, body).
fn descriptor(data: &[u8], tag: u8) -> Option<&[u8]> {
    if *data.first()? != tag {
        return None;
    }
    let mut len = 0usize;
    let mut at = 1;
    // The length takes one to four bytes, seven bits each.
    for _ in 0..4 {
        let b = *data.get(at)?;
        at += 1;
        len = (len << 7) | usize::from(b & 0x7f);
        if b & 0x80 == 0 {
            break;
        }
    }
    data.get(at..at + len)
}
