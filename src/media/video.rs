//! Preparing a video for upload: dropping what the camera wrote about where and when
//! it was recorded, without touching the pictures or sound.
//!
//! MP4 and QuickTime files point at their samples by absolute position, so nothing
//! may move. Boxes of metadata (`udta`, `meta`, and XMP in a `uuid` box) are renamed
//! `free`, which every player skips, and the recording times are zeroed in place.

/// Boxes that only hold metadata: where it was filmed, the phone, the software.
const METADATA: [&[u8; 4]; 3] = [b"udta", b"meta", b"uuid"];
/// Boxes with nothing but other boxes inside, to look through.
const CONTAINERS: [&[u8; 4]; 3] = [b"moov", b"trak", b"mdia"];
/// Headers that record when the file was made and changed.
const TIMES: [&[u8; 4]; 3] = [b"mvhd", b"tkhd", b"mdhd"];

/// Strips metadata from an MP4 or QuickTime file in place. Anything else is left
/// as it is.
pub fn strip(data: &mut [u8]) {
    strip_in(data, 0, data.len());
}

fn strip_in(data: &mut [u8], start: usize, end: usize) {
    for b in boxes(data, start, end) {
        let kind = &b.kind;
        if METADATA.contains(&kind) {
            data[b.head + 4..b.head + 8].copy_from_slice(b"free");
        } else if CONTAINERS.contains(&kind) {
            strip_in(data, b.body, b.end);
        } else if TIMES.contains(&kind) {
            zero_times(&mut data[b.body..b.end]);
        }
    }
}

/// Zeroes a header's creation and modification times, 32 or 64 bits by version.
fn zero_times(body: &mut [u8]) {
    let span = if body.first() == Some(&1) {
        4..20
    } else {
        4..12
    };
    if let Some(times) = body.get_mut(span) {
        times.fill(0);
    }
}

struct Box {
    kind: [u8; 4],
    /// Where the box starts, its contents start, and it ends.
    head: usize,
    body: usize,
    end: usize,
}

/// The boxes directly inside `data[start..end]`.
fn boxes(data: &[u8], start: usize, end: usize) -> Vec<Box> {
    let mut out = Vec::new();
    let mut at = start;
    while let Some(head) = data.get(at..at.saturating_add(8)).filter(|_| at + 8 <= end) {
        let size32 = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
        let kind = [head[4], head[5], head[6], head[7]];
        let (size, header) = match size32 {
            0 => (end - at, 8),
            1 => {
                let Some(big) = data.get(at + 8..at + 16) else {
                    break;
                };
                let mut b = [0u8; 8];
                b.copy_from_slice(big);
                (
                    usize::try_from(u64::from_be_bytes(b)).unwrap_or(usize::MAX),
                    16,
                )
            }
            n => (n as usize, 8),
        };
        let Some(stop) = at.checked_add(size).filter(|s| size >= header && *s <= end) else {
            break;
        };
        out.push(Box {
            kind,
            head: at,
            body: at + header,
            end: stop,
        });
        at = stop;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mp4_box(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
        let mut b = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        b.extend_from_slice(&kind);
        b.extend_from_slice(body);
        b
    }

    #[test]
    fn location_and_times_go_and_nothing_moves() {
        let place = mp4_box(*b"\xa9xyz", b"+40.5-074.4/");
        let udta = mp4_box(*b"udta", &place);
        let mut mvhd = vec![0u8; 100];
        mvhd[4..12].copy_from_slice(&[9; 8]);
        mvhd[12..16].copy_from_slice(&600u32.to_be_bytes());
        let mut tkhd = vec![0u8; 84];
        tkhd[4..12].copy_from_slice(&[7; 8]);
        let trak = mp4_box(*b"trak", &[mp4_box(*b"tkhd", &tkhd), udta.clone()].concat());
        let moov = mp4_box(*b"moov", &[mp4_box(*b"mvhd", &mvhd), trak, udta].concat());
        let mdat = mp4_box(*b"mdat", b"pictures");
        let mut file = [mp4_box(*b"ftyp", b"isom"), moov, mdat].concat();
        let before = file.clone();

        strip(&mut file);
        assert_eq!(file.len(), before.len());
        assert!(!file.windows(4).any(|w| w == b"udta"));
        assert!(
            file.windows(4).any(|w| w == b"\xa9xyz"),
            "hidden, not moved"
        );
        assert!(!file.windows(8).any(|w| w == [9; 8] || w == [7; 8]));
        // The timescale and the samples are untouched.
        assert!(file.windows(4).any(|w| w == 600u32.to_be_bytes()));
        assert!(file.ends_with(b"mdatpictures"));
        // The file still reads the same way.
        assert_eq!(
            crate::api::probe_video(&file).duration,
            crate::api::probe_video(&before).duration
        );
    }

    #[test]
    fn other_files_and_bad_sizes_are_left_alone() {
        let mut webm = b"\x1aE\xdf\xa3 not an mp4".to_vec();
        let copy = webm.clone();
        strip(&mut webm);
        assert_eq!(webm, copy);
        let mut bad = [&u32::MAX.to_be_bytes()[..], b"moov", b"udta"].concat();
        let copy = bad.clone();
        strip(&mut bad);
        assert_eq!(bad, copy);
    }
}
