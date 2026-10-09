//! Buttons, from the main action on a screen down to a post's like count.

use super::feedback::tip;
use super::icon::{Icon, icon};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, radius, size};
use iced::widget::{Button, MouseArea, button, mouse_area, row, text};
use iced::{Alignment, Element};

/// A text button that gains a soft background on hover.
pub fn ghost<'a, M: Clone + 'a>(label: impl text::IntoFragment<'a>, msg: M) -> Button<'a, M> {
    button(text(label).size(size::SMALL))
        .padding([gap::XS, gap::S])
        .style(theme::ghost)
        .on_press(msg)
}

/// The main action on a screen.
pub fn primary<'a, M: 'a>(label: impl text::IntoFragment<'a>) -> Button<'a, M> {
    button(text(label).size(size::BODY).font(theme::SEMIBOLD))
        .padding([gap::S, gap::L + gap::XS])
        .style(theme::primary)
}

/// The main action, with a leading icon.
pub fn primary_icon<'a, M: 'a>(i: Icon, label: &'a str) -> Button<'a, M> {
    button(
        row![
            icon(i, icon_size::M, Tone::OnAccent),
            text(label).size(size::BODY).font(theme::SEMIBOLD),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center),
    )
    .padding([gap::S, gap::L + gap::XS])
    .style(theme::primary)
}

/// A quieter filled button.
pub fn secondary<'a, M: 'a>(label: impl text::IntoFragment<'a>) -> Button<'a, M> {
    button(text(label).size(size::SMALL).font(theme::MEDIUM))
        .padding([gap::S - 1.0, gap::L])
        .style(theme::secondary)
}

pub fn danger<'a, M: Clone + 'a>(label: impl text::IntoFragment<'a>, msg: M) -> Button<'a, M> {
    button(text(label).size(size::SMALL).font(theme::MEDIUM))
        .padding([gap::S - 1.0, gap::L])
        .style(theme::danger)
        .on_press(msg)
}

/// An icon-only action with its name in a tooltip, for title bars and headers.
pub fn icon_action<'a, M: Clone + 'a>(i: Icon, label: &'a str, msg: M) -> Element<'a, M> {
    tip(icon_button(i, msg), label)
}

/// The same for something destructive, such as Delete or Leave.
pub fn danger_icon_action<'a, M: Clone + 'a>(i: Icon, label: &'a str, msg: M) -> Element<'a, M> {
    tip(
        round(icon(i, icon_size::L, Tone::Danger)).on_press(msg),
        label,
    )
}

/// A round icon-only button.
pub fn icon_button<'a, M: Clone + 'a>(i: Icon, msg: M) -> Button<'a, M> {
    round(icon(i, icon_size::L, Tone::Muted)).on_press(msg)
}

/// A round icon-only button for something that is on or off, tinted while it is on.
pub fn icon_toggle<'a, M: Clone + 'a>(i: Icon, on: bool, msg: M) -> Button<'a, M> {
    let tint = if on { Tone::Accent } else { Tone::Muted };
    round(icon(i, icon_size::L, tint)).on_press(msg)
}

fn round<'a, M: 'a>(content: Element<'a, M>) -> Button<'a, M> {
    button(content).padding(gap::S - 1.0).style(|t, s| {
        let mut st = theme::ghost(t, s);
        st.border = st.border.rounded(radius::PILL);
        st
    })
}

/// A post action: icon plus optional count. Without a message it is shown but can't
/// be pressed.
pub fn action<'a, M: Clone + 'a>(
    i: Icon,
    count: Option<String>,
    active: bool,
    tint: Tone,
    msg: Option<M>,
) -> Element<'a, M> {
    let fg = if active { tint } else { Tone::Faint };
    let mut content = row![icon(i, icon_size::M, fg)]
        .spacing(gap::XS + 2.0)
        .align_y(Alignment::Center);
    if let Some(c) = count {
        content = content.push(text(c).size(size::SMALL).style(theme::tone(fg)));
    }
    button(content)
        .padding([gap::XS + 1.0, gap::S + 2.0])
        .style(theme::action(active, tint))
        .on_press_maybe(msg)
        .into()
}

/// The floating "new posts" button over the top of a timeline.
pub fn new_posts<'a, M: Clone + 'a>(label: String, msg: M) -> Element<'a, M> {
    button(
        row![
            icon(Icon::ArrowUp, icon_size::S, Tone::OnAccent),
            text(label).size(size::SMALL).font(theme::MEDIUM),
        ]
        .spacing(gap::XS + 2.0)
        .align_y(Alignment::Center),
    )
    .padding([gap::XS + 2.0, gap::M + 2.0])
    .style(theme::primary)
    .on_press(msg)
    .into()
}

/// A clickable area that shows the hand cursor, for avatars, images and cards.
pub fn hand<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> MouseArea<'a, M> {
    mouse_area(content).interaction(iced::mouse::Interaction::Pointer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Save,
        Like,
        More,
    }

    fn clicked(view: Element<'_, Msg>, label: &str) -> Vec<Msg> {
        let mut ui = simulator(view);
        assert!(ui.click(label).is_ok(), "no {label:?} to click");
        ui.into_messages().collect()
    }

    #[test]
    fn buttons_send_their_message() {
        assert_eq!(
            clicked(ghost("Save", Msg::Save).into(), "Save"),
            [Msg::Save]
        );
        assert_eq!(
            clicked(danger("Delete", Msg::Save).into(), "Delete"),
            [Msg::Save]
        );
        assert_eq!(
            clicked(new_posts("3 new".into(), Msg::More), "3 new"),
            [Msg::More]
        );
        let like = action(
            Icon::Heart,
            Some("12".into()),
            false,
            Tone::Rose,
            Some(Msg::Like),
        );
        assert_eq!(clicked(like, "12"), [Msg::Like]);
    }

    #[test]
    fn actions_without_a_message_do_nothing() {
        let view = action::<Msg>(Icon::Heart, Some("12".into()), false, Tone::Rose, None);
        assert!(clicked(view, "12").is_empty());
        let plain: Element<'_, Msg> = primary("Post").into();
        assert!(clicked(plain, "Post").is_empty());
    }
}
