//! Pop-up menus: a floating list of items, such as a post's "…" menu.

use super::icon::{Icon, icon};
use super::surface::divider;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::text::IntoFragment;
use iced::widget::{Column, Row, Space, button, container, row, text};
use iced::{Alignment, Element, Length};

/// A floating menu holding `items`.
pub fn menu<'a, M: 'a>(items: Vec<Element<'a, M>>) -> Element<'a, M> {
    container(Column::with_children(items).spacing(gap::HAIR))
        .padding(gap::XS)
        .width(232)
        .style(theme::overlay)
        .into()
}

fn menu_row<'a, M: 'a>(i: Icon, label: impl IntoFragment<'a>, tint: Tone) -> Row<'a, M> {
    row![
        icon(i, icon_size::M, tint),
        text(label).size(size::SMALL).style(theme::tone(tint))
    ]
    .spacing(gap::M - 2.0)
    .align_y(Alignment::Center)
}

/// One menu item. Without a message it is greyed out and can't be pressed.
pub fn menu_item<'a, M: Clone + 'a>(
    i: Icon,
    label: impl IntoFragment<'a>,
    msg: Option<M>,
) -> Element<'a, M> {
    let tint = if msg.is_some() {
        Tone::Text
    } else {
        Tone::Faint
    };
    button(menu_row(i, label, tint))
        .width(Length::Fill)
        .padding([gap::S, gap::M - 2.0])
        .style(theme::row_button)
        .on_press_maybe(msg)
        .into()
}

/// A menu item that is on or off, with a check mark when it is on.
pub fn check_item<'a, M: Clone + 'a>(label: &'a str, on: bool, msg: M) -> Element<'a, M> {
    let mark: Element<'a, M> = if on {
        icon(Icon::Check, icon_size::M, Tone::Accent)
    } else {
        Space::new().width(16).height(16).into()
    };
    button(
        row![
            mark,
            text(label).size(size::SMALL).style(theme::tone(Tone::Text))
        ]
        .spacing(gap::M - 2.0)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([gap::S, gap::M - 2.0])
    .style(theme::row_button)
    .on_press(msg)
    .into()
}

/// A menu item for something destructive, such as Delete or Block.
pub fn danger_item<'a, M: Clone + 'a>(i: Icon, label: &'a str, msg: M) -> Element<'a, M> {
    button(menu_row(i, label, Tone::Danger))
        .width(Length::Fill)
        .padding([gap::S, gap::M - 2.0])
        .style(theme::row_button)
        .on_press(msg)
        .into()
}

/// A line between groups of items.
pub fn menu_divider<'a, M: 'a>() -> Element<'a, M> {
    container(divider()).padding([gap::XXS, 0.0]).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Copy,
        Delete,
        Toggle,
    }

    #[test]
    fn items_send_their_message_and_disabled_ones_do_not() {
        let view = || {
            menu(vec![
                menu_item(Icon::Copy, "Copy", Some(Msg::Copy)),
                menu_item(Icon::Quote, "Quoting is off", None),
                menu_divider(),
                danger_item(Icon::Trash, "Delete", Msg::Delete),
                check_item("Hide reposts", true, Msg::Toggle),
            ])
        };
        for (label, want) in [
            ("Copy", vec![Msg::Copy]),
            ("Quoting is off", vec![]),
            ("Delete", vec![Msg::Delete]),
            ("Hide reposts", vec![Msg::Toggle]),
        ] {
            let mut ui = simulator(view());
            assert!(ui.click(label).is_ok(), "no {label:?}");
            assert_eq!(ui.into_messages().collect::<Vec<_>>(), want);
        }
    }
}
