//! The blue check beside a verified account's name.

use super::icon::{Icon, icon};
use super::tip;
use crate::theme::Tone;
use iced::Element;

/// A verified account's check, or a trusted verifier's rosette when `verifier`, at
/// `px` and with a tooltip saying which.
pub fn verified<'a, M: 'a>(verifier: bool, px: f32) -> Element<'a, M> {
    let (i, label) = if verifier {
        (
            Icon::Verifier,
            "Trusted verifier: can verify other accounts",
        )
    } else {
        (Icon::Verified, "Verified account")
    };
    tip(icon(i, px, Tone::Verified), label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[test]
    fn both_checks_draw() {
        for verifier in [false, true] {
            let view: Element<'_, ()> = verified(verifier, 16.0);
            assert!(simulator(view).snapshot(&iced::Theme::Dark).is_ok());
        }
    }
}
