//! BiFrost's themes: the design tokens and the style functions built from them.
//!
//! Every colour, spacing step, radius and text size the UI uses comes from here, so
//! the look can be tuned in one place. Each theme is one file holding a [`Palette`]
//! (`dark.rs`, `light.rs`); `system.rs` follows the OS setting. Nothing here is
//! global: the app builds a [`Look`] from its settings and passes it to views, and
//! style functions read the palette from the iced theme they are drawn with.
//!
//! Adding a theme means adding its file and one line in [`PALETTES`].

mod dark;
mod light;
mod style;
pub mod system;
mod tokens;

pub use dark::DARK;
pub use light::LIGHT;
pub use style::*;
pub use tokens::{
    BOLD, FONT, FONT_FAMILY, MEDIUM, Palette, SEMIBOLD, Tone, fonts, gap, icon_size, layout,
    radius, size,
};

use crate::platform::fonts::SANS_FAMILY;
use iced::theme::Base;
use iced::{Font, Theme};
use std::sync::LazyLock;

/// Every theme. Style functions find their palette here by the iced theme's name.
pub const PALETTES: [&Palette; 2] = [&DARK, &LIGHT];

/// One iced theme per palette, built once.
static THEMES: LazyLock<Vec<Theme>> = LazyLock::new(|| {
    PALETTES
        .iter()
        .map(|p| {
            Theme::custom(
                p.name,
                iced::theme::Palette {
                    background: p.surface,
                    text: p.text,
                    primary: p.accent,
                    success: p.accent,
                    warning: p.warning,
                    danger: p.danger,
                },
            )
        })
        .collect()
});

/// The palette an iced theme was built from. Dark for a theme BiFrost didn't build.
pub fn palette(theme: &Theme) -> &'static Palette {
    PALETTES
        .iter()
        .find(|p| p.name == theme.name())
        .copied()
        .unwrap_or(&DARK)
}

/// What views need to know about the current look.
#[derive(Debug, Clone, Copy)]
pub struct Look {
    pub palette: &'static Palette,
    /// Post text in the system's proportional font instead of JetBrains Mono.
    pub sans_reading: bool,
}

impl Look {
    /// The iced theme to draw with.
    pub fn theme(self) -> Theme {
        THEMES
            .iter()
            .find(|t| t.name() == self.palette.name)
            .cloned()
            .unwrap_or(Theme::Dark)
    }

    /// The font and size for post text at `size`. The proportional font runs smaller
    /// than the monospace one at the same size, so it gets a point more.
    pub fn reading(self, size: f32) -> (Font, f32) {
        if self.sans_reading {
            (Font::with_name(SANS_FAMILY), size + 1.0)
        } else {
            (FONT, size)
        }
    }

    pub fn color(self, tone: Tone) -> iced::Color {
        self.palette.get(tone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_theme_finds_its_own_palette() {
        for p in PALETTES {
            let look = Look {
                palette: p,
                sans_reading: false,
            };
            assert_eq!(palette(&look.theme()), p);
        }
    }

    #[test]
    fn theme_names_are_unique() {
        let mut names: Vec<_> = PALETTES.iter().map(|p| p.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PALETTES.len());
    }

    #[test]
    fn sans_reading_runs_a_point_larger() {
        let look = Look {
            palette: &DARK,
            sans_reading: true,
        };
        let (font, size) = look.reading(13.0);
        assert_eq!(font, Font::with_name(SANS_FAMILY));
        assert!((size - 14.0).abs() < f32::EPSILON);
        let mono = Look {
            sans_reading: false,
            ..look
        };
        let (font, size) = mono.reading(13.0);
        assert_eq!(font, FONT);
        assert!((size - 13.0).abs() < f32::EPSILON);
    }

    #[test]
    fn publication_colours_need_to_read() {
        assert!((contrast([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 0.01);
        let pale = Some(([200, 200, 200], [255, 255, 255]));
        let strong = Some(([20, 60, 120], [255, 255, 255]));
        let theme = Look {
            palette: &DARK,
            sans_reading: false,
        }
        .theme();
        let active = iced::widget::button::Status::Active;
        assert_eq!(branded(pale)(&theme, active), primary(&theme, active));
        assert_ne!(branded(strong)(&theme, active), primary(&theme, active));
    }
}
