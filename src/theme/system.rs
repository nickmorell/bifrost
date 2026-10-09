//! "Match system": follows the OS light or dark setting, including changes while
//! BiFrost is open. iced reads the setting on each OS; this only maps it to a theme.

use super::tokens::Palette;
use super::{dark::DARK, light::LIGHT};
use iced::theme::Mode;
use iced::{Subscription, Task};

/// The theme for the OS setting. Dark when the OS doesn't say.
pub fn palette(mode: Mode) -> &'static Palette {
    if mode == Mode::Light { &LIGHT } else { &DARK }
}

/// The OS setting at startup.
pub fn current() -> Task<Mode> {
    iced::system::theme()
}

/// The OS setting whenever it changes.
pub fn changes() -> Subscription<Mode> {
    iced::system::theme_changes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_the_os_and_defaults_to_dark() {
        assert_eq!(palette(Mode::Light), &LIGHT);
        assert_eq!(palette(Mode::Dark), &DARK);
        assert_eq!(palette(Mode::None), &DARK);
    }
}
