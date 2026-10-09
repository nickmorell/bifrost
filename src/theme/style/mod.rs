//! The style functions widgets are drawn with. Each reads the palette from the iced
//! theme it is given, so the same view draws correctly in every theme.

mod buttons;
mod inputs;
mod structure;
mod surfaces;
mod text;

pub use buttons::*;
pub use inputs::*;
pub use structure::*;
pub use surfaces::*;
pub use text::*;

use super::tokens::Palette;
use iced::widget::button;
use iced::{Background, Border, Color, Shadow, Vector};

/// A wash over a surface: white on the dark palette, black on the light one.
fn ink(p: &Palette, a: f32) -> Color {
    Color { a, ..p.ink }
}

/// White, for washes that always sit on something dark (the image viewer, video).
fn white(a: f32) -> Color {
    Color::from_rgba(1.0, 1.0, 1.0, a)
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

fn rounded(r: f32) -> Border {
    Border::default().rounded(r)
}

fn soft_shadow(blur: f32, y: f32, a: f32) -> Shadow {
    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, a),
        offset: Vector::new(0.0, y),
        blur_radius: blur,
    }
}

/// `over` drawn on top of the opaque colour `under`.
fn mix(under: Color, over: Color) -> Color {
    let a = over.a;
    Color::from_rgb(
        under.r + (over.r - under.r) * a,
        under.g + (over.g - under.g) * a,
        under.b + (over.b - under.b) * a,
    )
}

fn button_base(bg: Option<Color>, fg: Color, r: f32) -> button::Style {
    button::Style {
        background: bg.map(Background::Color),
        text_color: fg,
        border: rounded(r),
        shadow: Shadow::default(),
        snap: true,
    }
}
