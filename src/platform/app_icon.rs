//! BiFrost's app icon as the pixels an OS takes, for the window and the tray. The
//! PNGs are exported from the SVGs in `assets/brand` (see `docs/brand.md`); the
//! Windows program icon and the macOS app bundle use the `.ico` and `.icns` files
//! there instead.

use image::ImageFormat;

/// 128 px, for the window: title bar, task switcher, and the taskbar or dock on Linux.
const WINDOW: &[u8] = include_bytes!("../../assets/brand/png/bifrost-128.png");

/// 32 px with a bolder mark, which stays readable when the tray shrinks it to 16 px.
const TRAY: &[u8] = include_bytes!("../../assets/brand/tray.png");

/// A square icon as rows of RGBA pixels, `side` by `side`.
pub struct Pixels {
    pub rgba: Vec<u8>,
    pub side: u32,
}

/// The window's icon, or `None` if it can't be decoded.
pub fn window() -> Option<Pixels> {
    decode(WINDOW)
}

/// The tray's icon without a badge, or `None` if it can't be decoded.
pub(in crate::platform) fn tray() -> Option<Pixels> {
    decode(TRAY)
}

fn decode(png: &[u8]) -> Option<Pixels> {
    let picture = image::load_from_memory_with_format(png, ImageFormat::Png)
        .ok()?
        .into_rgba8();
    (picture.width() == picture.height()).then(|| Pixels {
        side: picture.width(),
        rgba: picture.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_decodes_to_its_size() {
        let window = window().unwrap();
        assert_eq!(window.side, 128);
        assert_eq!(window.rgba.len(), 128 * 128 * 4);
        assert_eq!(tray().unwrap().side, 32);
    }

    #[test]
    fn the_corners_are_rounded_off() {
        assert_eq!(window().unwrap().rgba[3], 0);
    }
}
