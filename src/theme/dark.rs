//! The dark theme: BiFrost's default.

use super::tokens::{Palette, rgb, white};
use iced::Color;

pub const DARK: Palette = Palette {
    name: "BiFrost Dark",
    base: rgb(0x151517),
    surface: rgb(0x1e1e21),
    raised: rgb(0x27272b),
    overlay: rgb(0x2f2f34),
    hover: white(0.05),
    pressed: white(0.09),
    hairline: white(0.06),
    text: rgb(0xe9e9ec),
    muted: white(0.62),
    // 50% keeps small text at 4.5:1 or better on every surface (WCAG AA).
    faint: white(0.50),
    accent: rgb(0xa8d5ba),
    on_accent: rgb(0x13201a),
    accent_soft: Color::from_rgba(0.659, 0.835, 0.729, 0.14),
    rose: rgb(0xf2a9ba),
    danger: rgb(0xeba3a3),
    warning: rgb(0xe9cf9a),
    ink: Color::WHITE,
};
