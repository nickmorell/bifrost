//! Reading ISO BMFF boxes: a 4-byte size, a 4-byte type, then the body.

/// The boxes in `data`, as (type, body).
pub(super) fn boxes(data: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    let mut at = 0;
    std::iter::from_fn(move || {
        let rest = data.get(at..)?;
        let kind: [u8; 4] = rest.get(4..8)?.try_into().ok()?;
        let header = if u32_at(rest, 0)? == 1 { 16 } else { 8 };
        let len = box_len(rest);
        let body = rest.get(header..len)?;
        at += len;
        Some((kind, body))
    })
}

/// A box's whole length, header included: a 32-bit size, a 64-bit one when that is 1,
/// or to the end when it is 0.
pub(super) fn box_len(data: &[u8]) -> usize {
    match u32_at(data, 0) {
        Some(0) => data.len(),
        Some(1) => u64_at(data, 8)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(data.len()),
        Some(n) => (n as usize).max(8),
        None => data.len(),
    }
}

pub(super) fn child(data: &[u8], kind: [u8; 4]) -> Option<&[u8]> {
    boxes(data).find(|(k, _)| *k == kind).map(|(_, b)| b)
}

pub(super) fn children<'a>(data: &'a [u8], kind: &'a [u8; 4]) -> impl Iterator<Item = &'a [u8]> {
    boxes(data).filter(move |(k, _)| k == kind).map(|(_, b)| b)
}

pub(super) fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

pub(super) fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

pub(super) fn u64_at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(d.get(at..at + 8)?.try_into().ok()?))
}
