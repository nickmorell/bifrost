//! BiFrost's mark: two arcs, a bridge, in the accent colour (see `docs/brand.md`).

use crate::theme::{self, Tone};
use iced::Element;
use iced::widget::svg;
use std::sync::LazyLock;

const MARK: &[u8] = include_bytes!("../../../assets/brand/mark.svg");

/// Width over height of the mark's drawing.
const ASPECT: f32 = 860.0 / 389.0;

/// Created once so iced can cache the rasterised mark.
static HANDLE: LazyLock<svg::Handle> = LazyLock::new(|| svg::Handle::from_memory(MARK));

/// The mark, `width` wide, tinted with the theme's accent.
pub fn brand_mark<'a, M: 'a>(width: f32) -> Element<'a, M> {
    svg(HANDLE.clone())
        .width(width)
        .height(width / ASPECT)
        .style(theme::icon(Tone::Accent))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_is_the_svg_from_assets() {
        let doc = std::str::from_utf8(MARK).unwrap();
        assert!(doc.starts_with("<svg") && doc.contains("viewBox=\"82 315 860 389\""));
    }
}
