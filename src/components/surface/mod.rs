//! What content sits on: cards, hairlines, scroll areas and the reading column.

use crate::theme::{self, gap, size};
use iced::widget::{Container, container, row, rule, scrollable, text};
use iced::{Alignment, Element, Length};

/// A card inside the panel.
pub fn card<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Container<'a, M> {
    container(content)
        .padding(gap::M)
        .style(theme::raised)
        .width(Length::Fill)
}

/// A hairline outlined card.
pub fn outlined<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Container<'a, M> {
    container(content)
        .padding(gap::M)
        .style(theme::outlined)
        .width(Length::Fill)
}

/// A hairline between list items, inset from the panel's sides.
pub fn divider<'a, M: 'a>() -> Element<'a, M> {
    container(rule::horizontal(1).style(theme::hairline))
        .padding([0.0, gap::XL])
        .into()
}

/// A hairline across the full width.
pub fn divider_flush<'a, M: 'a>() -> Element<'a, M> {
    rule::horizontal(1).style(theme::hairline).into()
}

/// A full-width hairline with a short label in its middle, marking a place in a list.
pub fn marker<'a, M: 'a>(label: &'a str) -> Element<'a, M> {
    let line = || rule::horizontal(1).style(theme::accent_rule);
    row![
        line(),
        text(label)
            .size(size::CAPTION)
            .style(theme::tone(theme::Tone::Accent))
            .wrapping(text::Wrapping::None),
        line(),
    ]
    .spacing(gap::S)
    .align_y(Alignment::Center)
    .padding([gap::XS, gap::L])
    .into()
}

/// A vertical scroll area with BiFrost's thin scrollbar.
pub fn scroll<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> scrollable::Scrollable<'a, M> {
    scrollable(content)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(6)
                .scroller_width(6)
                .margin(3),
        ))
        .style(theme::scrollbar)
        .height(Length::Fill)
        .width(Length::Fill)
}

/// A centred column at reading width.
pub fn reading<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(
        container(content)
            .max_width(theme::layout::READING)
            .width(Length::Fill),
    )
    .center_x(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::{column, text};
    use iced_test::simulator;

    #[test]
    fn surfaces_show_their_content() {
        let view: Element<'_, ()> = reading(scroll(column![
            card(text("In a card")),
            divider(),
            outlined(text("Outlined")),
            divider_flush(),
            marker("You left off here"),
        ]));
        let mut ui = simulator(view);
        assert!(ui.find("In a card").is_ok());
        assert!(ui.find("Outlined").is_ok());
        assert!(ui.find("You left off here").is_ok());
    }
}
