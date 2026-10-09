//! The Privacy card's switches.

use super::{Msg, Privacy};
use crate::components::{self as ui, status};
use crate::theme::{self, gap, size};
use iced::widget::{column, row, text, toggler};
use iced::{Alignment, Element, Length};

impl Privacy {
    pub fn view(&self) -> Element<'_, Msg> {
        let now = match &self.now {
            None => return status("Loading…"),
            Some(Err(e)) => {
                return row![
                    ui::muted(format!("Couldn't read your privacy settings: {e}"))
                        .width(Length::Fill),
                    ui::secondary("Try again").on_press(Msg::Load),
                ]
                .spacing(gap::M)
                .align_y(Alignment::Center)
                .into();
            }
            Some(Ok(now)) => now,
        };
        column![
            switch(
                "Keep my posts out of Discover",
                "Asks Discover and other algorithmic feeds not to recommend your posts. People who follow you still see them.",
                now.hide_from_discover,
                Msg::Discover,
            ),
            switch(
                "Hide my posts from people who aren't signed in",
                "Bluesky's app and website won't show your profile and posts to logged-out visitors. Other apps may not honour this.",
                now.hide_logged_out,
                Msg::LoggedOut,
            ),
            ui::faint(
                "Your posts stay public either way: Bluesky has no private accounts, and anyone signed in can see them."
            ),
        ]
        .spacing(gap::L)
        .into()
    }
}

fn switch<'a>(
    title: &'static str,
    about: &'static str,
    on: bool,
    change: fn(bool) -> Msg,
) -> Element<'a, Msg> {
    row![
        column![
            text(title).size(size::BODY).font(theme::MEDIUM),
            ui::muted(about),
        ]
        .spacing(gap::XS)
        .width(Length::Fill),
        toggler(on)
            .size(size::TOGGLE)
            .style(theme::switch)
            .on_toggle(change),
    ]
    .spacing(gap::L)
    .align_y(Alignment::Center)
    .into()
}
