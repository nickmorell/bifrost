//! The tray icon's picture, drawn in code: a "B" on a disc in BiFrost's accent colour,
//! with a badge counting what is unread. Drawn here rather than loaded so the count can
//! change; colours are the theme's accent and its danger red, as plain pixels because
//! the OS takes raw RGBA.

/// Width and height in pixels.
pub(in crate::platform) const SIZE: usize = 32;

const ACCENT: [u8; 4] = [0xa8, 0xd5, 0xba, 0xff];
const INK: [u8; 4] = [0x1e, 0x1f, 0x22, 0xff];
const BADGE: [u8; 4] = [0xe0, 0x6c, 0x75, 0xff];
const BADGE_INK: [u8; 4] = [0xff, 0xff, 0xff, 0xff];

/// "B", 5 by 7.
const LETTER: [&str; 7] = [
    "11110", "10001", "10001", "11110", "10001", "10001", "11110",
];

/// Digits 0-9 and "+", 3 by 5.
const DIGITS: [[&str; 5]; 11] = [
    ["111", "101", "101", "101", "111"],
    ["010", "110", "010", "010", "111"],
    ["111", "001", "111", "100", "111"],
    ["111", "001", "111", "001", "111"],
    ["101", "101", "111", "001", "001"],
    ["111", "100", "111", "001", "111"],
    ["111", "100", "111", "101", "111"],
    ["111", "001", "010", "010", "010"],
    ["111", "101", "111", "101", "111"],
    ["111", "101", "111", "001", "111"],
    ["000", "010", "111", "010", "000"],
];

/// The icon as RGBA rows, with a badge when `unread` is above zero: the number up to
/// nine, "+" beyond.
pub(in crate::platform) fn pixels(unread: u64) -> Vec<u8> {
    let mut px = vec![0; SIZE * SIZE * 4];
    disc(&mut px, 15.5, 15.5, 15.5, ACCENT);
    glyph(&mut px, &LETTER, 3, 9, 6, INK);
    if unread > 0 {
        disc(&mut px, 24.0, 24.0, 8.0, BADGE);
        let digit = usize::try_from(unread).map_or(10, |n| n.min(10));
        glyph(&mut px, &DIGITS[digit], 2, 21, 19, BADGE_INK);
    }
    px
}

/// The text shown when hovering over the icon.
pub(super) fn tooltip(unread: u64) -> String {
    match unread {
        0 => "BiFrost".into(),
        1 => "BiFrost: 1 unread".into(),
        n => format!("BiFrost: {n} unread"),
    }
}

fn put(px: &mut [u8], x: usize, y: usize, colour: [u8; 4]) {
    if x < SIZE && y < SIZE {
        let i = (y * SIZE + x) * 4;
        px[i..i + 4].copy_from_slice(&colour);
    }
}

fn disc(px: &mut [u8], cx: f32, cy: f32, r: f32, colour: [u8; 4]) {
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            if dx * dx + dy * dy <= r * r {
                put(px, x, y, colour);
            }
        }
    }
}

/// Draws a bitmap glyph, each of its cells `scale` pixels square, from (`left`, `top`).
fn glyph(px: &mut [u8], rows: &[&str], scale: usize, left: usize, top: usize, colour: [u8; 4]) {
    for (row, line) in rows.iter().enumerate() {
        for (col, cell) in line.bytes().enumerate() {
            if cell == b'1' {
                for dy in 0..scale {
                    for dx in 0..scale {
                        put(px, left + col * scale + dx, top + row * scale + dy, colour);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(px: &[u8], x: usize, y: usize) -> [u8; 4] {
        let i = (y * SIZE + x) * 4;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    }

    #[test]
    fn the_icon_is_square_rgba() {
        assert_eq!(pixels(0).len(), SIZE * SIZE * 4);
    }

    #[test]
    fn a_badge_shows_only_when_something_is_unread() {
        assert_eq!(at(&pixels(0), 19, 27), ACCENT);
        assert_eq!(at(&pixels(3), 19, 27), BADGE);
        assert_ne!(pixels(3), pixels(4));
        assert_eq!(pixels(10), pixels(250));
    }

    #[test]
    fn corners_stay_transparent() {
        assert_eq!(at(&pixels(5), 0, 0)[3], 0);
    }

    #[test]
    fn the_tooltip_counts_what_is_unread() {
        assert_eq!(tooltip(0), "BiFrost");
        assert_eq!(tooltip(12), "BiFrost: 12 unread");
    }
}
