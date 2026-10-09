//! Text colours.

use crate::theme::palette;
use crate::theme::tokens::{ON_MEDIA, Tone};
use iced::Theme;
use iced::widget::{svg, text};

pub fn muted(theme: &Theme) -> text::Style {
    let p = palette(theme);
    text::Style {
        color: Some(p.muted),
    }
}

pub fn faint(theme: &Theme) -> text::Style {
    let p = palette(theme);
    text::Style {
        color: Some(p.faint),
    }
}

pub fn danger_text(theme: &Theme) -> text::Style {
    let p = palette(theme);
    text::Style {
        color: Some(p.danger),
    }
}

pub fn warning_text(theme: &Theme) -> text::Style {
    let p = palette(theme);
    text::Style {
        color: Some(p.warning),
    }
}

/// Text over photos and video.
pub fn on_media(_: &Theme) -> text::Style {
    text::Style {
        color: Some(ON_MEDIA),
    }
}

/// Text in `tone`, for text that isn't one of the named styles above.
pub fn tone(tone: Tone) -> impl Fn(&Theme) -> text::Style {
    move |theme| text::Style {
        color: Some(palette(theme).get(tone)),
    }
}

/// A colour someone picked for their name, such as in a live chat, drawn a little
/// toward the text colour so it reads on light and dark themes alike.
pub fn picked(rgb: [u8; 3]) -> impl Fn(&Theme) -> text::Style {
    move |theme| {
        let [r, g, b] = rgb;
        let own = iced::Color::from_rgba8(r, g, b, 0.75);
        text::Style {
            color: Some(super::mix(palette(theme).text, own)),
        }
    }
}

/// A symbolic icon tinted `tone`.
pub fn icon(tone: Tone) -> impl Fn(&Theme, svg::Status) -> svg::Style {
    move |theme, _| svg::Style {
        color: Some(palette(theme).get(tone)),
    }
}
