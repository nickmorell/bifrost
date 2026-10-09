//! The light theme.

use super::tokens::{Palette, black, rgb};
use iced::Color;

pub const LIGHT: Palette = Palette {
    name: "BiFrost Light",
    base: rgb(0xe6e8e4),
    surface: rgb(0xf8f8f6),
    raised: rgb(0xeeefeb),
    overlay: rgb(0xffffff),
    hover: black(0.04),
    pressed: black(0.08),
    hairline: black(0.09),
    text: rgb(0x1c1e1d),
    muted: black(0.66),
    // Also 4.5:1 or better on every light surface.
    faint: black(0.56),
    // The sage, darkened until it reads as text on white.
    accent: rgb(0x3d7a57),
    on_accent: rgb(0xffffff),
    accent_soft: Color::from_rgba(0.239, 0.478, 0.341, 0.12),
    rose: rgb(0xc0436a),
    danger: rgb(0xb3443c),
    warning: rgb(0x8a6618),
    ink: Color::BLACK,
};
