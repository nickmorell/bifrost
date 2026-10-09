//! Buttons.

use super::{alpha, button_base, ink, white};
use crate::theme::palette;
use crate::theme::tokens::{ON_MEDIA, Tone, radius};
use iced::widget::button;
use iced::{Border, Color, Theme};

/// Text-only button that gains a soft background on hover.
pub fn ghost(theme: &Theme, status: button::Status) -> button::Style {
    let p = palette(theme);
    match status {
        button::Status::Active => button_base(None, p.muted, radius::S),
        button::Status::Hovered => button_base(Some(p.hover), p.text, radius::S),
        button::Status::Pressed => button_base(Some(p.pressed), p.text, radius::S),
        button::Status::Disabled => button_base(None, p.faint, radius::S),
    }
}

/// An empty slot to fill, such as the deck's "Add column": an outline that fills in
/// on hover.
pub fn slot(theme: &Theme, status: button::Status) -> button::Style {
    let p = palette(theme);
    let (bg, text) = match status {
        button::Status::Hovered => (Some(p.hover), p.text),
        button::Status::Pressed => (Some(p.pressed), p.text),
        _ => (None, p.muted),
    };
    let mut s = button_base(bg, text, radius::L);
    s.border = Border {
        color: ink(p, 0.14),
        width: 1.0,
        radius: radius::L.into(),
    };
    s
}

/// Fully transparent button for wrapping rich content (names, rows).
pub fn bare(theme: &Theme, _: button::Status) -> button::Style {
    let p = palette(theme);
    button_base(None, p.text, 0.0)
}

/// The round play button over a video's still, and the buttons in its controls.
pub fn video_button(_: &Theme, status: button::Status) -> button::Style {
    let a = match status {
        button::Status::Hovered => 0.82,
        button::Status::Pressed => 0.9,
        _ => 0.6,
    };
    button_base(
        Some(Color::from_rgba(0.0, 0.0, 0.0, a)),
        Color::WHITE,
        radius::PILL,
    )
}

/// The previous / next buttons in the image viewer.
pub fn viewer_arrow(theme: &Theme, status: button::Status) -> button::Style {
    let p = palette(theme);
    match status {
        button::Status::Hovered => button_base(Some(white(0.12)), ON_MEDIA, radius::PILL),
        button::Status::Pressed => button_base(Some(white(0.18)), ON_MEDIA, radius::PILL),
        button::Status::Disabled => {
            let mut s = button_base(None, p.faint, radius::PILL);
            s.text_color = Color::TRANSPARENT;
            s
        }
        button::Status::Active => button_base(Some(white(0.06)), ON_MEDIA, radius::PILL),
    }
}

/// A post in a list: a faint wash on hover, so it reads as something to open. Square
/// and edge to edge, so it lines up with the dividers instead of looking like a
/// selected card.
pub fn post(theme: &Theme, status: button::Status) -> button::Style {
    let p = palette(theme);
    match status {
        button::Status::Hovered => button_base(Some(ink(p, 0.018)), p.text, 0.0),
        button::Status::Pressed => button_base(Some(ink(p, 0.04)), p.text, 0.0),
        _ => button_base(None, p.text, 0.0),
    }
}

/// A row that lights up on hover, e.g. a person in search results.
pub fn row_button(theme: &Theme, status: button::Status) -> button::Style {
    let p = palette(theme);
    match status {
        button::Status::Hovered => button_base(Some(p.hover), p.text, radius::M),
        button::Status::Pressed => button_base(Some(p.pressed), p.text, radius::M),
        _ => button_base(None, p.text, radius::M),
    }
}

/// The main call to action.
pub fn primary(theme: &Theme, status: button::Status) -> button::Style {
    let p = palette(theme);
    let bg = match status {
        button::Status::Active => p.accent,
        button::Status::Hovered => Color {
            a: 0.88,
            ..p.accent
        },
        button::Status::Pressed => Color {
            a: 0.78,
            ..p.accent
        },
        button::Status::Disabled => alpha(p.accent, 0.3),
    };
    let fg = if status == button::Status::Disabled {
        alpha(p.on_accent, 0.6)
    } else {
        p.on_accent
    };
    button_base(Some(bg), fg, radius::PILL)
}

/// A quieter filled button. Translucent, so it reads on any surface.
pub fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let p = palette(theme);
    let (bg, fg) = match status {
        button::Status::Active => (ink(p, 0.07), p.text),
        button::Status::Hovered => (ink(p, 0.11), p.text),
        button::Status::Pressed => (ink(p, 0.15), p.text),
        button::Status::Disabled => (ink(p, 0.04), p.faint),
    };
    button_base(Some(bg), fg, radius::PILL)
}

pub fn danger(theme: &Theme, status: button::Status) -> button::Style {
    let p = palette(theme);
    let a = match status {
        button::Status::Hovered => 0.26,
        button::Status::Pressed => 0.32,
        _ => 0.18,
    };
    button_base(Some(alpha(p.danger, a)), p.danger, radius::PILL)
}

/// Sidebar navigation item.
pub fn nav(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let p = palette(theme);
        let bg = match (active, status) {
            (true, _) => Some(p.pressed),
            (false, button::Status::Hovered) => Some(p.hover),
            (false, button::Status::Pressed) => Some(p.pressed),
            _ => None,
        };
        let fg = if active { p.text } else { p.muted };
        button_base(bg, fg, radius::M)
    }
}

/// A pill tab, e.g. feeds and profile sections.
pub fn pill(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let p = palette(theme);
        if active {
            return button_base(Some(p.accent_soft), p.accent, radius::PILL);
        }
        match status {
            button::Status::Hovered => button_base(Some(p.hover), p.text, radius::PILL),
            button::Status::Pressed => button_base(Some(p.pressed), p.text, radius::PILL),
            _ => button_base(None, p.muted, radius::PILL),
        }
    }
}

/// A post action (reply, repost, like). `tint` colours it while active.
pub fn action(active: bool, tint: Tone) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let p = palette(theme);
        let tint = p.get(tint);
        let fg = if active { tint } else { p.faint };
        match status {
            button::Status::Hovered => button_base(Some(alpha(tint, 0.10)), tint, radius::PILL),
            button::Status::Pressed => button_base(Some(alpha(tint, 0.16)), tint, radius::PILL),
            button::Status::Disabled => button_base(None, alpha(p.faint, 0.5), radius::PILL),
            button::Status::Active => button_base(None, fg, radius::PILL),
        }
    }
}

/// A publication's own button colours (`[r, g, b]` behind and in front), used only
/// when the two contrast at least 4.5 to 1; otherwise the theme's primary button.
pub fn branded(
    colors: Option<([u8; 3], [u8; 3])>,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    let readable = colors.filter(|(bg, fg)| contrast(*bg, *fg) >= 4.5);
    move |theme, status| match readable {
        Some((bg, fg)) => {
            let bg = Color::from_rgb8(bg[0], bg[1], bg[2]);
            let a = match status {
                button::Status::Active => 1.0,
                button::Status::Hovered => 0.88,
                button::Status::Pressed => 0.78,
                button::Status::Disabled => 0.3,
            };
            button_base(
                Some(Color { a, ..bg }),
                Color::from_rgb8(fg[0], fg[1], fg[2]),
                radius::PILL,
            )
        }
        None => primary(theme, status),
    }
}

/// The WCAG contrast ratio between two colours, from 1 to 21.
pub fn contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
    fn luminance([r, g, b]: [u8; 3]) -> f32 {
        let channel = |c: u8| {
            let c = f32::from(c) / 255.0;
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
    }
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}
