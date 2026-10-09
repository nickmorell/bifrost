//! What shows instead of a post, its media or a quoted post that moderation hides,
//! each with a Show button.

use super::super::button::ghost;
use super::super::icon::{Icon, icon};
use super::super::surface::{card, outlined};
use super::super::text::muted;
use crate::theme::icon_size;
use crate::theme::{Tone, gap};
use iced::widget::{container, row};
use iced::{Alignment, Element, Length};

/// A hidden post: why, and Show.
pub fn hidden<'a, M: Clone + 'a>(reason: String, show: M) -> Element<'a, M> {
    container(
        card(
            row![
                icon(Icon::Close, icon_size::S, Tone::Faint),
                muted(reason).width(Length::Fill),
                ghost("Show", show),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        )
        .padding([gap::S, gap::M]),
    )
    .padding([gap::M, gap::XL])
    .into()
}

/// A post's hidden images or video.
pub fn hidden_media<'a, M: Clone + 'a>(reason: &str, show: M) -> Element<'a, M> {
    card(
        row![
            icon(Icon::Photo, icon_size::M, Tone::Faint),
            muted(format!("Media hidden: {reason}")).width(Length::Fill),
            ghost("Show", show),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center),
    )
    .padding([gap::L, gap::M])
    .into()
}

/// A hidden quoted post.
pub fn hidden_quote<'a, M: Clone + 'a>(reason: &str, show: M) -> Element<'a, M> {
    outlined(
        row![
            icon(Icon::EyeOff, icon_size::M, Tone::Faint),
            muted(format!("Quoted post · {reason}")).width(Length::Fill),
            ghost("Show", show),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[test]
    fn show_reveals() {
        let views: [Element<'_, u8>; 3] = [
            hidden("Muted word".into(), 1),
            hidden_media("Nudity", 2),
            hidden_quote("Blocked", 3),
        ];
        for (want, view) in (1..).zip(views) {
            let mut ui = simulator(view);
            assert!(ui.click("Show").is_ok());
            assert_eq!(ui.into_messages().collect::<Vec<_>>(), [want]);
        }
    }
}
