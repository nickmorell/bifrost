//! Dividers and scrollbars.

use super::{ink, rounded};
use crate::theme::palette;
use crate::theme::tokens::radius;
use iced::widget::{container, rule, scrollable};
use iced::{Border, Color, Shadow, Theme};

pub fn hairline(theme: &Theme) -> rule::Style {
    let p = palette(theme);
    rule::Style {
        color: p.hairline,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

/// A hairline in the accent colour, for a marker in a list.
pub fn accent_rule(theme: &Theme) -> rule::Style {
    rule::Style {
        color: palette(theme).accent_soft,
        ..hairline(theme)
    }
}

/// Thin scrollbars that stay out of the way until hovered.
pub fn scrollbar(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let p = palette(theme);
    let active = matches!(
        status,
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Dragged { .. }
    );
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: ink(p, if active { 0.22 } else { 0.09 }).into(),
            border: rounded(radius::PILL),
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: p.overlay.into(),
            border: rounded(radius::PILL),
            shadow: Shadow::default(),
            icon: p.muted,
        },
    }
}

/// The position marker under a row of tabs: an accent sliver on a hairline track.
pub fn tab_indicator(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let p = palette(theme);
    let active = matches!(
        status,
        scrollable::Status::Hovered {
            is_horizontal_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Dragged { .. }
    );
    let mut style = scrollbar(theme, status);
    style.horizontal_rail = scrollable::Rail {
        background: Some(p.hairline.into()),
        border: rounded(radius::PILL),
        scroller: scrollable::Scroller {
            background: Color {
                a: if active { 1.0 } else { 0.8 },
                ..p.accent
            }
            .into(),
            border: rounded(radius::PILL),
        },
    };
    style
}
