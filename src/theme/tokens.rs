//! The names every view and style uses: colours (as [`Palette`] fields and [`Tone`]s),
//! spacing, radii, text sizes, layout widths and fonts. Only colours differ between
//! themes; everything else here is the same in all of them.

use iced::{Color, Font, font};

pub(super) const fn rgb(hex: u32) -> Color {
    Color::from_rgb8(
        (hex >> 16) as u8,
        (hex >> 8 & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

pub(super) const fn white(alpha: f32) -> Color {
    Color::from_rgba(1.0, 1.0, 1.0, alpha)
}

pub(super) const fn black(alpha: f32) -> Color {
    Color::from_rgba(0.0, 0.0, 0.0, alpha)
}

/// One theme's colours. Surfaces step up in lightness the closer they sit to the user.
#[derive(Debug, PartialEq)]
pub struct Palette {
    /// Identifies the theme; also the name of the iced theme built from it.
    pub name: &'static str,
    /// The window itself, behind the sidebar.
    pub base: Color,
    /// The floating panel that holds the current screen.
    pub surface: Color,
    /// Cards inside the panel: quotes, link cards, inputs.
    pub raised: Color,
    /// Things that float above everything: menus, dialogs, toasts.
    pub overlay: Color,
    pub hover: Color,
    pub pressed: Color,
    pub hairline: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    /// Soft sage: calm and clearly not a system blue.
    pub accent: Color,
    /// Text and icons drawn on top of `accent`.
    pub on_accent: Color,
    pub accent_soft: Color,
    /// Liked posts.
    pub rose: Color,
    pub danger: Color,
    pub warning: Color,
    /// Bluesky's verification checks, blue in every app.
    pub verified: Color,
    /// Washes over surfaces (hover, selection, tracks): white on dark, black on light.
    pub ink: Color,
}

/// Text and icons over photos and video, which stay light in every theme.
pub const ON_MEDIA: Color = rgb(0xe9e9ec);

/// Behind dialogs, in every theme.
pub const SCRIM: Color = black(0.55);

/// A colour picked by role rather than value, for views that hand a colour to a
/// widget. It is looked up in the current theme when the widget is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Text,
    Muted,
    Faint,
    Accent,
    OnAccent,
    AccentSoft,
    Rose,
    Danger,
    Warning,
    Verified,
    Surface,
    Raised,
    Overlay,
    OnMedia,
}

impl Palette {
    pub fn get(&self, tone: Tone) -> Color {
        match tone {
            Tone::Text => self.text,
            Tone::Muted => self.muted,
            Tone::Faint => self.faint,
            Tone::Accent => self.accent,
            Tone::OnAccent => self.on_accent,
            Tone::AccentSoft => self.accent_soft,
            Tone::Rose => self.rose,
            Tone::Danger => self.danger,
            Tone::Warning => self.warning,
            Tone::Verified => self.verified,
            Tone::Surface => self.surface,
            Tone::Raised => self.raised,
            Tone::Overlay => self.overlay,
            Tone::OnMedia => ON_MEDIA,
        }
    }
}

/// Spacing steps, in logical pixels.
pub mod gap {
    /// Between lines that belong together, such as a name over a handle.
    pub const HAIR: f32 = 1.0;
    pub const XXS: f32 = 2.0;
    pub const XS: f32 = 4.0;
    pub const S: f32 = 8.0;
    pub const M: f32 = 12.0;
    pub const L: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
}

/// Corner radii.
pub mod radius {
    pub const S: f32 = 6.0;
    pub const M: f32 = 10.0;
    pub const L: f32 = 14.0;
    pub const PILL: f32 = 999.0;
}

/// Text sizes. JetBrains Mono runs wide, so these sit a step below a proportional font.
pub mod size {
    pub const CAPTION: f32 = 11.0;
    pub const SMALL: f32 = 12.0;
    pub const BODY: f32 = 13.5;
    pub const LARGE: f32 = 15.5;
    pub const TITLE: f32 = 18.0;
    /// The title at the top of each screen.
    pub const HEADING: f32 = 22.0;
    pub const DISPLAY: f32 = 30.0;
    /// Emoji reactions, which are unreadable at text sizes.
    pub const EMOJI: f32 = 16.0;
    /// On/off switches and checkboxes.
    pub const TOGGLE: f32 = 20.0;
    pub const CHECKBOX: f32 = 16.0;
}

/// Icon sizes, from the smallest beside caption text to the play button over a video.
pub mod icon_size {
    pub const XXS: f32 = 12.0;
    /// Beside small text, such as a post's repost or pinned line.
    pub const XS: f32 = 13.0;
    pub const S: f32 = 14.0;
    pub const SM: f32 = 15.0;
    /// Beside body text, in menus and buttons.
    pub const M: f32 = 16.0;
    /// Round icon buttons and the sidebar.
    pub const L: f32 = 18.0;
    pub const XL: f32 = 20.0;
    pub const XXL: f32 = 22.0;
    /// The play button over a video.
    pub const HERO: f32 = 26.0;
    /// BiFrost's mark beside the name on the login screen.
    pub const BRAND: f32 = 44.0;
}

/// Layout constants.
pub mod layout {
    /// Width of the sidebar.
    pub const SIDEBAR: f32 = 248.0;
    /// The sidebar in a narrow window, icons only.
    pub const RAIL: f32 = 64.0;
    /// Gap between the panel and the window edge.
    pub const INSET: f32 = 8.0;
    /// Widest a column of posts gets, for comfortable line lengths.
    pub const READING: f32 = 680.0;
    /// Dialog widths: most dialogs, and ones holding a longer form or text.
    pub const DIALOG: f32 = 480.0;
    pub const DIALOG_WIDE: f32 = 560.0;
}

/// JetBrains Mono, built into the binary. These are Latin-only subsets without
/// ligatures (see `assets/fonts`); other scripts and emoji fall back to system fonts.
pub mod fonts {
    pub const REGULAR: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
    pub const MEDIUM: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Medium.ttf");
    pub const SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-SemiBold.ttf");
    pub const BOLD: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");
}

pub const FONT_FAMILY: &str = "JetBrains Mono";
pub const FONT: Font = Font::with_name(FONT_FAMILY);
pub const MEDIUM: Font = Font {
    weight: font::Weight::Medium,
    ..FONT
};
pub const SEMIBOLD: Font = Font {
    weight: font::Weight::Semibold,
    ..FONT
};
pub const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..FONT
};
