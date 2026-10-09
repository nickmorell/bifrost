//! Sidebar destinations and unread counts.

use super::icon::{Icon, icon};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{button, container, row, space, stack, text, tooltip};
use iced::{Alignment, Element, Length, Padding};

/// A sidebar destination. On the narrow `rail` it is just the icon, with a dot for
/// anything unread and the name on hover.
pub fn nav_item<'a, M: Clone + 'a>(
    i: Icon,
    label: &'a str,
    active: bool,
    badge: Option<u64>,
    msg: M,
    rail: bool,
) -> Element<'a, M> {
    let tint = if active { Tone::Accent } else { Tone::Muted };
    let badge = badge.filter(|n| *n > 0);
    if rail {
        let mut content = stack![container(icon(i, icon_size::L, tint)).center(Length::Fill)];
        if badge.is_some() {
            content = content.push(
                container(container(space::horizontal().width(7).height(7)).style(theme::badge))
                    .align_right(Length::Fill)
                    .padding(Padding::ZERO.right(gap::M - 2.0).top(gap::XS)),
            );
        }
        return tooltip(
            button(content)
                .width(Length::Fill)
                .height(36)
                .padding(0)
                .style(theme::nav(active))
                .on_press(msg),
            container(text(label).size(size::CAPTION)).padding([gap::XXS, gap::S]),
            tooltip::Position::Right,
        )
        .gap(gap::S)
        .style(theme::overlay)
        .into();
    }
    let mut content = row![
        icon(i, icon_size::L, tint),
        text(label)
            .size(size::BODY)
            .font(if active { theme::MEDIUM } else { theme::FONT })
            .width(Length::Fill),
    ]
    .spacing(gap::M)
    .align_y(Alignment::Center);
    if let Some(n) = badge {
        content = content.push(count_badge(n));
    }
    button(content)
        .width(Length::Fill)
        .padding([gap::S + 1.0, gap::M])
        .style(theme::nav(active))
        .on_press(msg)
        .into()
}

/// An unread count, capped at "99+".
pub fn count_badge<'a, M: 'a>(n: u64) -> Element<'a, M> {
    container(
        text(badge_label(n))
            .size(size::CAPTION)
            .font(theme::SEMIBOLD),
    )
    .padding([1.0, 7.0])
    .style(theme::badge)
    .into()
}

fn badge_label(n: u64) -> String {
    if n > 99 { "99+".into() } else { n.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[test]
    fn nav_items_open_their_screen_and_show_unread() {
        let view = nav_item(Icon::Bell, "Notifications", false, Some(120), "open", false);
        let mut ui = simulator(view);
        assert!(ui.find("99+").is_ok());
        assert!(ui.click("Notifications").is_ok());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), ["open"]);
        assert_eq!(badge_label(7), "7");
    }
}
