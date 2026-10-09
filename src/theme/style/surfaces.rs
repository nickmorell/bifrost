//! Containers: the window, panels, cards, overlays and media frames.

use super::{alpha, ink, mix, rounded, soft_shadow};
use crate::theme::palette;
use crate::theme::tokens::{ON_MEDIA, SCRIM, radius};
use iced::widget::container;
use iced::{Border, Color, Theme};

/// The window background.
pub fn base(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(p.base.into()),
        text_color: Some(p.text),
        ..Default::default()
    }
}

/// The floating panel that holds the current screen.
pub fn panel(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(p.surface.into()),
        border: Border {
            color: p.hairline,
            width: 1.0,
            radius: radius::L.into(),
        },
        shadow: soft_shadow(24.0, 6.0, 0.35),
        ..Default::default()
    }
}

/// A card inside the panel.
pub fn raised(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(p.raised.into()),
        border: rounded(radius::M),
        ..Default::default()
    }
}

/// A quieter card: a hairline outline and no fill.
pub fn outlined(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        border: Border {
            color: p.hairline,
            width: 1.0,
            radius: radius::M.into(),
        },
        ..Default::default()
    }
}

/// Menus, dialogs and toasts.
pub fn overlay(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(p.overlay.into()),
        text_color: Some(p.text),
        border: Border {
            color: p.hairline,
            width: 1.0,
            radius: radius::M.into(),
        },
        shadow: soft_shadow(28.0, 8.0, 0.45),
        ..Default::default()
    }
}

/// A dialog floating over the app, such as the composer.
pub fn dialog(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(p.surface.into()),
        text_color: Some(p.text),
        border: Border {
            color: p.hairline,
            width: 1.0,
            radius: radius::L.into(),
        },
        shadow: soft_shadow(48.0, 12.0, 0.6),
        ..Default::default()
    }
}

pub fn scrim(_: &Theme) -> container::Style {
    container::Style {
        background: Some(SCRIM.into()),
        ..Default::default()
    }
}

/// A placeholder block where an image is still loading.
pub fn placeholder(r: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let p = palette(theme);
        container::Style {
            background: Some(p.raised.into()),
            border: rounded(r),
            ..Default::default()
        }
    }
}

/// A placeholder for an image that is loading, a little lighter at `pulse` 1.
pub fn loading(r: f32, pulse: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let p = palette(theme);
        let base = p.raised;
        let wash = ink(p, 0.035 * pulse);
        container::Style {
            background: Some(mix(base, wash).into()),
            border: rounded(r),
            ..Default::default()
        }
    }
}

/// A small count bubble, e.g. unread notifications.
pub fn badge(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(p.accent.into()),
        text_color: Some(p.on_accent),
        border: rounded(radius::PILL),
        ..Default::default()
    }
}

/// The red LIVE mark on live streams and the avatars of people streaming.
pub fn live_badge(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(p.danger.into()),
        text_color: Some(ON_MEDIA),
        border: rounded(radius::S),
        ..Default::default()
    }
}

/// The red ring around the avatar of someone streaming now.
pub fn live_ring(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        border: Border {
            color: p.danger,
            width: 2.0,
            radius: radius::PILL.into(),
        },
        ..Default::default()
    }
}

/// Highlights unread notifications.
pub fn unread(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(alpha(p.accent, 0.05).into()),
        border: rounded(radius::M),
        ..Default::default()
    }
}

/// The line joining posts in a conversation.
pub fn thread_line(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(ink(p, 0.12).into()),
        border: rounded(1.0),
        ..Default::default()
    }
}

/// A bar in a chart.
pub fn bar(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(p.accent.into()),
        border: rounded(1.0),
        ..Default::default()
    }
}

/// A chart's bar for nothing, drawn faintly so the days still line up.
pub fn bar_empty(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(ink(p, 0.08).into()),
        border: rounded(1.0),
        ..Default::default()
    }
}

/// A key name in the shortcuts list.
pub fn keycap(theme: &Theme) -> container::Style {
    let p = palette(theme);
    container::Style {
        background: Some(p.overlay.into()),
        text_color: Some(p.text),
        border: Border {
            color: p.hairline,
            width: 1.0,
            radius: radius::S.into(),
        },
        ..Default::default()
    }
}

/// The post picked with the keyboard gets a quiet accent outline.
pub fn selected(theme: &Theme, on: bool) -> container::Style {
    let p = palette(theme);
    if !on {
        return container::Style::default();
    }
    container::Style {
        background: Some(ink(p, 0.025).into()),
        // Square and edge to edge, like the post hover, and strong enough to find at a
        // glance after a j or k.
        border: Border {
            color: alpha(p.accent, 0.8),
            width: 1.5,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

/// The small GIF / ALT labels over media.
pub fn media_badge(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.72).into()),
        text_color: Some(Color::WHITE),
        border: Border::default().rounded(radius::S - 2.0),
        ..Default::default()
    }
}

/// Behind a playing video, so letterboxing is black.
pub fn video_frame(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::BLACK.into()),
        border: Border::default().rounded(radius::M),
        ..Default::default()
    }
}

/// The bar of controls along the bottom of a video.
pub fn video_controls(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.6).into()),
        text_color: Some(Color::WHITE),
        ..Default::default()
    }
}

/// Behind the image viewer: nearly black, so the picture stands out.
pub fn viewer_backdrop(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba(0.04, 0.04, 0.05, 0.985).into()),
        text_color: Some(ON_MEDIA),
        ..Default::default()
    }
}
