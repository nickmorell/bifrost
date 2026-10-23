//! Reading a video's picture size and length from its MP4 or QuickTime header, so
//! the composer can check it and the post can carry its aspect ratio.

use super::VideoInfo;

/// Reads the size and length from an MP4 or QuickTime file. Anything else gives
/// an empty [`VideoInfo`].
pub fn probe(bytes: &[u8]) -> VideoInfo {
    let mut info = VideoInfo::default();
    let Some(moov) = find_box(bytes, *b"moov") else {
        return info;
    };
    if let Some(mvhd) = find_box(moov, *b"mvhd") {
        info.duration = read_mvhd(mvhd);
    }
    for trak in boxes(moov)
        .filter(|(kind, _)| kind == b"trak")
        .map(|(_, b)| b)
    {
        let is_video = find_box(trak, *b"mdia")
            .and_then(|m| find_box(m, *b"hdlr"))
            .is_some_and(|h| h.get(8..12) == Some(b"vide"));
        if let Some(tkhd) = find_box(trak, *b"tkhd")
            && is_video
            && let Some(size) = read_tkhd(tkhd)
        {
            info.size = Some(size);
            break;
        }
    }
    info
}

/// The boxes directly inside `data`: (type, contents).
fn boxes(data: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    let mut pos = 0usize;
    std::iter::from_fn(move || {
        let head = data.get(pos..pos + 8)?;
        let mut size = u32::from_be_bytes(head[0..4].try_into().ok()?) as usize;
        let kind: [u8; 4] = head[4..8].try_into().ok()?;
        let mut header = 8;
        if size == 1 {
            size = u64::from_be_bytes(data.get(pos + 8..pos + 16)?.try_into().ok()?) as usize;
            header = 16;
        } else if size == 0 {
            size = data.len() - pos;
        }
        if size < header {
            return None;
        }
        let body = data.get(pos + header..pos.checked_add(size)?)?;
        pos += size;
        Some((kind, body))
    })
}

fn find_box(data: &[u8], kind: [u8; 4]) -> Option<&[u8]> {
    boxes(data).find(|(k, _)| *k == kind).map(|(_, b)| b)
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

fn read_mvhd(b: &[u8]) -> Option<f64> {
    let (scale, duration) = if *b.first()? == 1 {
        (be32(b, 20)?, be64(b, 24)?)
    } else {
        (be32(b, 12)?, u64::from(be32(b, 16)?))
    };
    (scale > 0).then(|| duration as f64 / f64::from(scale))
}

fn read_tkhd(b: &[u8]) -> Option<(u32, u32)> {
    // The matrix starts after the version-dependent times.
    let matrix = if *b.first()? == 1 { 52 } else { 40 };
    let w = be32(b, matrix + 36)? >> 16;
    let h = be32(b, matrix + 40)? >> 16;
    if w == 0 || h == 0 {
        return None;
    }
    // A matrix with a non-zero `b` turns the picture a quarter, as phones record
    // portrait video.
    let turned = be32(b, matrix + 4)? != 0;
    Some(if turned { (h, w) } else { (w, h) })
}

#[cfg(test)]
mod tests {
    use super::super::{is_video_name, upload_name};
    use super::*;

    fn mp4_box(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
        let mut b = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        b.extend_from_slice(&kind);
        b.extend_from_slice(body);
        b
    }

    fn tkhd(w: u32, h: u32, turned: bool) -> Vec<u8> {
        let mut b = vec![0u8; 40];
        let mut matrix = [0u32; 9];
        if turned {
            matrix[1] = 0x0001_0000;
            matrix[3] = 0xffff_0000;
        } else {
            matrix[0] = 0x0001_0000;
            matrix[4] = 0x0001_0000;
        }
        matrix[8] = 0x4000_0000;
        for m in matrix {
            b.extend_from_slice(&m.to_be_bytes());
        }
        b.extend_from_slice(&(w << 16).to_be_bytes());
        b.extend_from_slice(&(h << 16).to_be_bytes());
        mp4_box(*b"tkhd", &b)
    }

    fn trak(kind: [u8; 4], w: u32, h: u32, turned: bool) -> Vec<u8> {
        let mut hdlr = vec![0u8; 8];
        hdlr.extend_from_slice(&kind);
        hdlr.extend_from_slice(&[0; 12]);
        let mdia = mp4_box(*b"mdia", &mp4_box(*b"hdlr", &hdlr));
        let mut body = tkhd(w, h, turned);
        body.extend(mdia);
        mp4_box(*b"trak", &body)
    }

    #[test]
    fn reads_size_and_length() {
        let mut mvhd = vec![0u8; 12];
        mvhd.extend_from_slice(&1000u32.to_be_bytes());
        mvhd.extend_from_slice(&12_500u32.to_be_bytes());
        mvhd.extend_from_slice(&[0; 80]);
        let mut moov = mp4_box(*b"mvhd", &mvhd);
        moov.extend(trak(*b"soun", 0, 0, false));
        moov.extend(trak(*b"vide", 1920, 1080, true));
        let mut file = mp4_box(*b"ftyp", b"isom\0\0\0\0");
        file.extend(mp4_box(*b"moov", &moov));
        let info = probe(&file);
        assert_eq!(info.duration, Some(12.5));
        // Recorded on its side, so shown portrait.
        assert_eq!(info.size, Some((1080, 1920)));
        assert_eq!(probe(b"not a video"), VideoInfo::default());
        assert!(is_video_name("Clip.MOV"));
        assert!(!is_video_name("cat.gif"));
        assert_eq!(upload_name("my clip (1).mp4"), "myclip1.mp4");
    }
}
