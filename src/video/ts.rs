//! Pulling H.264 video and AAC audio out of MPEG transport stream segments, the
//! format HLS video segments come in.

use std::collections::HashMap;

const PACKET: usize = 188;
pub const SYNC: u8 = 0x47;
const STREAM_H264: u8 = 0x1b;
const STREAM_AAC: u8 = 0x0f;
/// Video ticks run at 90 kHz.
const TICKS: f64 = 90_000.0;

/// Seconds into the video of a time in ticks, given (segment start, its first tick).
pub(super) fn time_of((start, first): (f64, u64), ticks: u64) -> f64 {
    // Ticks wrap at 2^33; differences within a segment stay small.
    let diff = (ticks.wrapping_sub(first) & ((1 << 33) - 1)) as i64;
    let diff = if diff > 1 << 32 {
        diff - (1 << 33)
    } else {
        diff
    };
    start + diff as f64 / TICKS
}

/// One PES packet: its presentation time (90 kHz ticks) and payload.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub pts: Option<u64>,
    pub data: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Demuxed {
    /// H.264 access units in Annex B form, in decode order.
    pub video: Vec<Unit>,
    /// AAC in ADTS frames.
    pub audio: Vec<Unit>,
    /// The video is in a format the player cannot decode, such as HEVC.
    pub unsupported_video: bool,
}

#[derive(Default)]
struct Pes {
    pts: Option<u64>,
    data: Vec<u8>,
    started: bool,
}

/// Splits a transport stream into its video and audio.
pub fn demux(data: &[u8]) -> Demuxed {
    let mut out = Demuxed::default();
    let mut pmt_pid = None;
    let mut video_pid = None;
    let mut audio_pid = None;
    let mut pes: HashMap<u16, Pes> = HashMap::new();

    let start = data.iter().position(|&b| b == SYNC).unwrap_or(data.len());
    for packet in data[start..].chunks_exact(PACKET) {
        if packet[0] != SYNC {
            continue;
        }
        let unit_start = packet[1] & 0x40 != 0;
        let pid = (u16::from(packet[1] & 0x1f) << 8) | u16::from(packet[2]);
        let adaptation = (packet[3] >> 4) & 0x3;
        let mut offset = 4;
        if adaptation & 0x2 != 0 {
            offset += 1 + packet[4] as usize;
        }
        if adaptation & 0x1 == 0 || offset >= PACKET {
            continue;
        }
        let payload = &packet[offset..];

        if pid == 0 && unit_start {
            pmt_pid = parse_pat(payload).or(pmt_pid);
        } else if Some(pid) == pmt_pid && unit_start {
            for (kind, es) in parse_pmt(payload) {
                match kind {
                    STREAM_H264 => video_pid = Some(es),
                    STREAM_AAC => audio_pid = Some(es),
                    0x24 | 0x27 => out.unsupported_video = true,
                    _ => {}
                }
            }
        } else if Some(pid) == video_pid || Some(pid) == audio_pid {
            let entry = pes.entry(pid).or_default();
            if unit_start {
                let done = std::mem::take(entry);
                flush(&mut out, pid, video_pid, done);
                let (pts, body) = parse_pes_header(payload);
                entry.pts = pts;
                entry.data.extend_from_slice(body);
                entry.started = true;
            } else if entry.started {
                entry.data.extend_from_slice(payload);
            }
        }
    }
    for (pid, p) in pes {
        flush(&mut out, pid, video_pid, p);
    }
    out
}

fn flush(out: &mut Demuxed, pid: u16, video_pid: Option<u16>, p: Pes) {
    if !p.started || p.data.is_empty() {
        return;
    }
    let unit = Unit {
        pts: p.pts,
        data: p.data,
    };
    if Some(pid) == video_pid {
        out.video.push(unit);
    } else {
        out.audio.push(unit);
    }
}

/// The PMT's PID from a PAT section.
fn parse_pat(payload: &[u8]) -> Option<u16> {
    let s = section(payload)?;
    let len = ((usize::from(s[1] & 0x0f)) << 8) | usize::from(s[2]);
    let end = (3 + len).saturating_sub(4).min(s.len());
    let mut i = 8;
    while i + 4 <= end {
        let program = (u16::from(s[i]) << 8) | u16::from(s[i + 1]);
        let pid = (u16::from(s[i + 2] & 0x1f) << 8) | u16::from(s[i + 3]);
        if program != 0 {
            return Some(pid);
        }
        i += 4;
    }
    None
}

/// (stream type, PID) for each stream a PMT section lists.
fn parse_pmt(payload: &[u8]) -> Vec<(u8, u16)> {
    let Some(s) = section(payload) else {
        return Vec::new();
    };
    if s.len() < 12 {
        return Vec::new();
    }
    let len = ((usize::from(s[1] & 0x0f)) << 8) | usize::from(s[2]);
    let end = (3 + len).saturating_sub(4).min(s.len());
    let info_len = ((usize::from(s[10] & 0x0f)) << 8) | usize::from(s[11]);
    let mut i = 12 + info_len;
    let mut out = Vec::new();
    while i + 5 <= end {
        let kind = s[i];
        let pid = (u16::from(s[i + 1] & 0x1f) << 8) | u16::from(s[i + 2]);
        let es_len = ((usize::from(s[i + 3] & 0x0f)) << 8) | usize::from(s[i + 4]);
        out.push((kind, pid));
        i += 5 + es_len;
    }
    out
}

fn section(payload: &[u8]) -> Option<&[u8]> {
    let pointer = *payload.first()? as usize;
    let s = payload.get(1 + pointer..)?;
    (s.len() >= 8).then_some(s)
}

/// The PTS and the start of the payload of a PES packet.
fn parse_pes_header(p: &[u8]) -> (Option<u64>, &[u8]) {
    if p.len() < 9 || p[0..3] != [0, 0, 1] {
        return (None, p);
    }
    let header_len = p[8] as usize;
    let body = p.get(9 + header_len..).unwrap_or_default();
    let pts = (p[7] & 0x80 != 0 && p.len() >= 14).then(|| read_pts(&p[9..14]));
    (pts, body)
}

fn read_pts(b: &[u8]) -> u64 {
    (u64::from((b[0] >> 1) & 0x07) << 30)
        | (u64::from(b[1]) << 22)
        | (u64::from(b[2] >> 1) << 15)
        | (u64::from(b[3]) << 7)
        | u64::from(b[4] >> 1)
}

/// One ADTS frame's header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adts {
    /// MPEG-4 audio object type (2 is AAC-LC).
    pub object_type: u8,
    pub rate_index: u8,
    pub channels: u8,
    pub header_len: usize,
    pub frame_len: usize,
}

impl Adts {
    pub fn sample_rate(&self) -> u32 {
        const RATES: [u32; 13] = [
            96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350,
        ];
        RATES
            .get(self.rate_index as usize)
            .copied()
            .unwrap_or(44100)
    }

    /// The two-byte AudioSpecificConfig decoders take.
    pub fn config(&self) -> [u8; 2] {
        let v = (u16::from(self.object_type) << 11)
            | (u16::from(self.rate_index) << 7)
            | (u16::from(self.channels) << 3);
        v.to_be_bytes()
    }
}

pub fn parse_adts(b: &[u8]) -> Option<Adts> {
    if b.len() < 7 || b[0] != 0xff || b[1] & 0xf0 != 0xf0 {
        return None;
    }
    let protection_absent = b[1] & 0x01 != 0;
    let frame_len =
        (usize::from(b[3] & 0x03) << 11) | (usize::from(b[4]) << 3) | usize::from(b[5] >> 5);
    let header_len = if protection_absent { 7 } else { 9 };
    if frame_len < header_len {
        return None;
    }
    Some(Adts {
        object_type: (b[2] >> 6) + 1,
        rate_index: (b[2] >> 2) & 0x0f,
        channels: ((b[2] & 0x01) << 2) | (b[3] >> 6),
        header_len,
        frame_len,
    })
}

/// Splits ADTS data into frames: (header, raw AAC payload).
pub fn adts_frames(data: &[u8]) -> Vec<(Adts, &[u8])> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 7 <= data.len() {
        match parse_adts(&data[i..]) {
            Some(h) if i + h.frame_len <= data.len() => {
                out.push((h, &data[i + h.header_len..i + h.frame_len]));
                i += h.frame_len;
            }
            Some(_) => break,
            None => i += 1,
        }
    }
    out
}

/// Skips an ID3 tag at the start of packed audio segments.
pub fn skip_id3(data: &[u8]) -> &[u8] {
    if data.len() >= 10 && &data[0..3] == b"ID3" {
        let size = data[6..10]
            .iter()
            .fold(0usize, |acc, b| (acc << 7) | usize::from(b & 0x7f));
        return data.get(10 + size..).unwrap_or_default();
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(pid: u16, start: bool, payload: &[u8]) -> Vec<u8> {
        let mut p = vec![
            SYNC,
            ((pid >> 8) as u8 & 0x1f) | if start { 0x40 } else { 0 },
            pid as u8,
        ];
        let room = PACKET - 4;
        if payload.len() < room {
            // Pad with an adaptation field.
            let pad = room - payload.len();
            p.push(0x30);
            p.push((pad - 1) as u8);
            if pad > 1 {
                p.push(0);
                p.extend(std::iter::repeat_n(0xff, pad - 2));
            }
        } else {
            p.push(0x10);
        }
        p.extend_from_slice(payload);
        assert_eq!(p.len(), PACKET);
        p
    }

    fn pes(pts: u64, body: &[u8]) -> Vec<u8> {
        let mut p = vec![0, 0, 1, 0xe0, 0, 0, 0x80, 0x80, 5];
        p.push((((pts >> 30) as u8 & 0x07) << 1) | 0x21);
        p.push((pts >> 22) as u8);
        p.push((((pts >> 15) as u8) << 1) | 1);
        p.push((pts >> 7) as u8);
        p.push(((pts as u8) << 1) | 1);
        p.extend_from_slice(body);
        p
    }

    #[test]
    fn demuxes_video_and_audio() {
        let pat = [
            0, 0x00, 0xb0, 13, 0, 1, 0xc1, 0, 0, 0, 1, 0xf0, 0x00, 0, 0, 0, 0,
        ];
        let pmt = [
            0, 0x02, 0xb0, 23, 0, 1, 0xc1, 0, 0, 0xe1, 0x00, 0xf0, 0x00, 0x1b, 0xe1, 0x00, 0xf0,
            0x00, 0x0f, 0xe1, 0x01, 0xf0, 0x00, 0, 0, 0, 0,
        ];
        let mut ts = Vec::new();
        ts.extend(packet(0, true, &pat));
        ts.extend(packet(0x1000, true, &pmt));
        let big: Vec<u8> = (0..300u32).map(|i| i as u8).collect();
        let v = pes(90_000, &big);
        ts.extend(packet(0x100, true, &v[..184]));
        ts.extend(packet(0x100, false, &v[184..]));
        ts.extend(packet(
            0x101,
            true,
            &pes(90_000 + 3000, &[0xff, 0xf1, 1, 2]),
        ));
        ts.extend(packet(0x100, true, &pes(93_003, &[9, 9])));
        let d = demux(&ts);
        assert_eq!(d.video.len(), 2);
        assert_eq!(d.video[0].pts, Some(90_000));
        assert_eq!(d.video[0].data, big);
        assert_eq!(d.video[1].data, vec![9, 9]);
        assert_eq!(d.audio.len(), 1);
        assert_eq!(d.audio[0].pts, Some(93_000));
    }

    #[test]
    fn reads_adts_headers() {
        // AAC-LC, 44.1 kHz, stereo, 10-byte frame.
        let frame = [0xff, 0xf1, 0x50, 0x80, 0x01, 0x5f, 0xfc, 1, 2, 3];
        let h = parse_adts(&frame).unwrap();
        assert_eq!(h.object_type, 2);
        assert_eq!(h.sample_rate(), 44100);
        assert_eq!(h.channels, 2);
        assert_eq!(h.frame_len, 10);
        assert_eq!(h.config(), [0x12, 0x10]);
        let mut two = frame.to_vec();
        two.extend_from_slice(&frame);
        let frames = adts_frames(&two);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].1, &[1, 2, 3]);
    }
}
