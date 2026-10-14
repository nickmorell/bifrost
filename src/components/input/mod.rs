//! Text fields and drop-down lists.

use crate::theme::{self, gap, size};
use iced::Length;
use iced::widget::{PickList, TextInput, pick_list, text_input};

/// A text field.
pub fn field<'a, M: Clone + 'a>(placeholder: &str, value: &str) -> TextInput<'a, M> {
    text_input(placeholder, value)
        .size(size::BODY)
        .padding([gap::S + 2.0, gap::M])
        .style(theme::input)
}

/// Rows a drop-down list shows before it scrolls.
const MENU_ROWS: usize = 10;
/// How tall the menu of a longer drop-down list is.
const MENU_HEIGHT: f32 = 320.0;

/// A drop-down list. Its menu is as tall as its options, up to [`MENU_HEIGHT`].
pub fn select<'a, T, M: Clone + 'a>(
    options: &'a [T],
    selected: Option<T>,
    on_select: impl Fn(T) -> M + 'a,
) -> PickList<'a, T, &'a [T], T, M>
where
    T: ToString + PartialEq + Clone,
{
    pick_list(options, selected, on_select)
        .text_size(size::SMALL)
        .padding([gap::S - 1.0, gap::M])
        .width(150)
        .menu_height(if options.len() > MENU_ROWS {
            Length::Fixed(MENU_HEIGHT)
        } else {
            Length::Shrink
        })
        .style(theme::select)
        .menu_style(theme::select_menu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Element;
    use iced_test::simulator;

    #[test]
    fn fields_report_what_is_typed() {
        let view: Element<'_, String> = field("Search", "").on_input(|s| s).into();
        let mut ui = simulator(view);
        assert!(ui.click("Search").is_ok());
        let _ = ui.typewrite("rust");
        assert_eq!(ui.into_messages().last().as_deref(), Some("rust"));
    }

    #[test]
    fn selects_draw() {
        let options = ["Dark", "Light"];
        let view: Element<'_, &str> = select(&options, Some("Light"), |s| s).into();
        assert!(simulator(view).snapshot(&iced::Theme::Dark).is_ok());
    }
}
