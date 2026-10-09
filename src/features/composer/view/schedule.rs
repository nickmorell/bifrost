//! The Schedule button and its menu: quick times and a typed one.

use super::{Host, View};
use crate::components::{self as ui, Icon, icon_button, popover};
use crate::features::composer::message::Msg;
use crate::features::composer::schedule;
use crate::theme::{self, gap, size};
use iced::widget::{Column, button, column, container, row, text};
use iced::{Alignment, Element, Length};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    pub(super) fn schedule_button(&self, can_post: bool) -> Element<'a, M> {
        let c = self.composer;
        let lift = self.lift;
        let usable = can_post && c.video.is_none() && !c.threaded();
        let base = ui::tip(
            icon_button(Icon::Calendar, self.msg(Msg::ToggleSchedule))
                .on_press_maybe(usable.then(|| self.msg(Msg::ToggleSchedule))),
            "Schedule. It's sent then if BiFrost is running, or waiting in the tray.",
        );
        let popup = c.schedule_open.then(|| {
            let mut col = Column::new()
                .spacing(gap::HAIR)
                .push(container(ui::faint("Send it later")).padding([gap::XS, gap::M - 2.0]));
            for (label, at) in schedule::presets(chrono::Local::now()) {
                col = col.push(
                    button(text(label).size(size::SMALL))
                        .width(Length::Fill)
                        .padding([gap::S, gap::M - 2.0])
                        .style(theme::row_button)
                        .on_press(self.msg(Msg::Schedule(at))),
                );
            }
            let typed = column![
                ui::faint("Or a time, like 2026-10-09 14:30"),
                row![
                    ui::field("YYYY-MM-DD HH:MM", &c.schedule_text)
                        .on_input(move |t| lift(Msg::ScheduleText(t)))
                        .on_submit(self.msg(Msg::ScheduleTyped)),
                    ui::secondary("Schedule").on_press(self.msg(Msg::ScheduleTyped)),
                ]
                .spacing(gap::S)
                .align_y(Alignment::Center),
            ]
            .spacing(gap::XS)
            .padding([gap::S, gap::M - 2.0]);
            container(col.push(typed))
                .padding(gap::XS)
                .width(300)
                .style(theme::overlay)
                .into()
        });
        popover(base, popup, self.msg(Msg::CloseMenus)).into()
    }
}
