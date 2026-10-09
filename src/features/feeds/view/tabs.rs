//! The Home tabs: one per pinned feed, and a button to pin more.

use crate::components::{self as ui, Icon, icon_button, pill};
use crate::features::feeds::{Feeds, Msg};
use crate::theme::gap;
use iced::widget::{Row, container, row};
use iced::{Alignment, Element, Length, Padding};

pub struct Tabs<'a> {
    pub feeds: &'a Feeds,
}

impl<'a> Tabs<'a> {
    pub fn view(self) -> Element<'a, Msg> {
        let mut tabs = Row::new().spacing(gap::XS);
        for (i, tab) in self.feeds.tabs.iter().enumerate() {
            tabs = tabs.push(pill(&tab.name, i == self.feeds.active, Msg::Select(i)));
        }
        ui::reading(
            row![
                container(ui::tab_strip("feed-tabs", tabs)).width(Length::Fill),
                ui::tip(icon_button(Icon::Plus, Msg::Discover), "Pin more feeds"),
            ]
            .spacing(gap::XS)
            .align_y(Alignment::Start)
            .padding(Padding::from([0.0, gap::L + gap::XS]).right(gap::L)),
        )
    }
}
