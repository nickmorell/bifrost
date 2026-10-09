//! Telling the person what is going on: status lines, errors with a way out, toasts
//! and tooltips.

use super::button::secondary;
use super::icon::{Icon, icon};
use super::text::{faint, muted};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Space, button, column, container, row, text, tooltip};
use iced::{Alignment, Element, Length, Padding};

/// A quiet line of status text: loading, empty, end of list.
pub fn status<'a, M: 'a>(content: impl text::IntoFragment<'a>) -> Element<'a, M> {
    container(faint(content).size(size::SMALL))
        .padding([gap::XL, gap::XL])
        .center_x(Length::Fill)
        .into()
}

/// Shows `label` in a small bubble when the pointer rests on `content`.
pub fn tip<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    label: impl text::IntoFragment<'a>,
) -> Element<'a, M> {
    tooltip(
        content,
        container(text(label).size(size::CAPTION))
            .padding([gap::XXS, gap::S])
            .max_width(360),
        tooltip::Position::Top,
    )
    .gap(gap::XS)
    .delay(std::time::Duration::from_millis(500))
    .style(theme::overlay)
    .into()
}

/// Something that failed to load: a warning icon, the reason as a sentence, and the
/// next step: "Sign in again" (`sign_in`) when the session has run out, "Try again"
/// (`try_again`) otherwise.
pub fn retry<'a, M: Clone + 'a>(
    why: impl Into<String>,
    sign_in: M,
    try_again: M,
) -> Element<'a, M> {
    let why = sentence(&why.into());
    let (why, action) = if signed_out(&why) {
        (
            "You've been signed out. Sign in again to carry on.".to_string(),
            secondary("Sign in again").on_press(sign_in),
        )
    } else {
        (why, secondary("Try again").on_press(try_again))
    };
    container(
        column![
            icon(Icon::Alert, icon_size::XL, Tone::Warning),
            muted(why)
                .size(size::BODY)
                .align_x(iced::alignment::Horizontal::Center),
            action,
        ]
        .spacing(gap::M)
        .max_width(420)
        .align_x(Alignment::Center),
    )
    .padding([gap::XL, gap::XL])
    .center_x(Length::Fill)
    .into()
}

fn signed_out(why: &str) -> bool {
    let l = why.to_lowercase();
    l.contains("session expired") || l.contains("log in again") || l.contains("sign in again")
}

/// Makes a raw error read as a sentence: a capital first letter and a full stop.
pub fn sentence(raw: &str) -> String {
    let raw = raw.trim();
    let mut chars = raw.chars();
    let mut out: String = match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => return "Something went wrong.".into(),
    };
    if !out.ends_with(['.', '!', '?']) {
        out.push('.');
    }
    out
}

/// A floating notice with a close button.
pub fn toast<'a, M: Clone + 'a>(message: &'a str, dismiss: M) -> Element<'a, M> {
    container(
        row![
            text(message).size(size::SMALL),
            Space::new().width(gap::S),
            button(icon(Icon::Close, icon_size::S, Tone::Faint))
                .padding(gap::XS)
                .style(theme::ghost)
                .on_press(dismiss),
        ]
        .align_y(Alignment::Center),
    )
    .padding(Padding::from([gap::S, gap::M]).left(gap::L))
    .max_width(560)
    .style(theme::overlay)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        SignIn,
        Again,
    }

    fn pressed(why: &str, label: &str) -> Vec<Msg> {
        let mut ui = simulator(retry(why, Msg::SignIn, Msg::Again));
        assert!(ui.click(label).is_ok(), "no {label:?} for {why:?}");
        ui.into_messages().collect()
    }

    #[test]
    fn errors_offer_the_right_next_step() {
        assert_eq!(pressed("timed out", "Try again"), [Msg::Again]);
        assert_eq!(
            pressed("session expired; please log in again", "Sign in again"),
            [Msg::SignIn]
        );
    }

    #[test]
    fn errors_read_as_sentences() {
        assert_eq!(sentence(" timed out "), "Timed out.");
        assert_eq!(sentence("Done!"), "Done!");
        assert_eq!(sentence(""), "Something went wrong.");
    }

    #[test]
    fn status_and_toasts_show_their_text() {
        assert!(simulator(status::<()>("Loading")).find("Loading").is_ok());
        assert!(simulator(toast("Saved", ())).find("Saved").is_ok());
    }
}
