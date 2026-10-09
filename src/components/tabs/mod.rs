//! Tabs: single pills, and a sideways-scrolling strip of them.

mod drag_scroll;

use crate::theme::{self, gap, size};
use drag_scroll::drag_scroll;
use iced::widget::{button, scrollable, text};
use iced::{Element, Length};

/// A pill-shaped tab.
pub fn pill<'a, M: Clone + 'a>(
    label: impl text::IntoFragment<'a>,
    active: bool,
    msg: M,
) -> Element<'a, M> {
    button(
        text(label)
            .size(size::SMALL)
            .font(if active { theme::MEDIUM } else { theme::FONT }),
    )
    .padding([gap::XS + 2.0, gap::M + 2.0])
    .style(theme::pill(active))
    .on_press(msg)
    .into()
}

/// A horizontally scrolling row of tabs with an accent marker underneath showing
/// where the visible part sits in the whole row. It scrolls with the wheel, the
/// marker, or by dragging the tabs themselves.
pub fn tab_strip<'a, M: 'a>(id: &'static str, tabs: impl Into<Element<'a, M>>) -> Element<'a, M> {
    const MARKER: f32 = 3.0;
    drag_scroll(
        id,
        MARKER + gap::S,
        scrollable(tabs)
            .id(id)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .width(MARKER)
                    .scroller_width(MARKER)
                    .spacing(gap::S),
            ))
            .style(theme::tab_indicator)
            .width(Length::Fill),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::row;
    use iced_test::simulator;

    #[test]
    fn tabs_in_a_strip_still_take_clicks() {
        let tabs = row![pill("Following", true, 1), pill("Rust folks", false, 2)];
        let mut ui = simulator(tab_strip("test-tabs", tabs));
        assert!(ui.click("Rust folks").is_ok());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [2]);
    }
}
