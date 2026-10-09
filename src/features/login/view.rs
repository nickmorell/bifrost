//! The login form.

use super::{Login, Msg};
use crate::components as ui;
use crate::theme::{self, gap, size};
use iced::widget::{column, container, row, space, text};
use iced::{Alignment, Element, Length};

impl Login {
    pub fn view(&self) -> Element<'_, Msg> {
        let mut form = column![
            row![
                container(space::horizontal().width(10).height(10)).style(theme::badge),
                text("bifrost").size(size::DISPLAY).font(theme::SEMIBOLD),
            ]
            .spacing(gap::M)
            .align_y(Alignment::Center),
            ui::muted("Sign in with your Bluesky account."),
            space::vertical().height(gap::S),
        ]
        .spacing(gap::M)
        .width(400);

        if let Some(n) = self.notice {
            form = form.push(text(n).size(size::SMALL).style(theme::warning_text));
        }

        form = form.push(
            ui::field("Handle or email, e.g. alice.bsky.social", &self.identifier)
                .on_input(Msg::Identifier)
                .on_submit(if self.password.is_empty() {
                    Msg::Browser
                } else {
                    Msg::Submit
                }),
        );

        let browser: Element<'_, Msg> = if self.browser {
            row![
                ui::muted("Finish signing in in your browser…").width(Length::Fill),
                ui::ghost("Cancel", Msg::CancelBrowser),
            ]
            .spacing(gap::M)
            .align_y(Alignment::Center)
            .into()
        } else {
            ui::primary("Sign in with your browser")
                .on_press_maybe((!self.busy).then_some(Msg::Browser))
                .into()
        };
        form = form.push(browser).push(
            ui::faint("Opens Bluesky's sign-in page, so BiFrost never sees your password.")
                .line_height(1.5),
        );

        let rule = || {
            container(iced::widget::rule::horizontal(1).style(theme::hairline)).width(Length::Fill)
        };
        form = form
            .push(space::vertical().height(gap::XS))
            .push(
                row![rule(), ui::faint("or with an app password"), rule()]
                    .spacing(gap::M)
                    .align_y(Alignment::Center),
            )
            .push(
                ui::field("App password (xxxx-xxxx-xxxx-xxxx)", &self.password)
                    .secure(true)
                    .on_input(Msg::Password)
                    .on_submit(Msg::Submit),
            );

        if let Some(code) = &self.code {
            form = form.push(
                ui::field("Email sign-in code", code)
                    .on_input(Msg::Code)
                    .on_submit(Msg::Submit),
            );
        }

        if self.advanced {
            form = form.push(ui::field("Hosting provider", &self.service).on_input(Msg::Service));
        }

        if let Some(e) = &self.error {
            form = form.push(text(e).size(size::SMALL).style(theme::danger_text));
        }

        let sign_in = ui::secondary(if self.busy && !self.browser {
            "Signing in…"
        } else {
            "Sign in"
        })
        .on_press_maybe((!self.busy).then_some(Msg::Submit));
        let advanced = ui::ghost(
            if self.advanced {
                "Hide advanced"
            } else {
                "Other hosting provider"
            },
            Msg::ToggleAdvanced,
        );

        form = form
            .push(
                row![sign_in, advanced]
                    .spacing(gap::M)
                    .align_y(Alignment::Center),
            )
            .push(space::vertical().height(gap::S))
            .push(
                ui::faint("Create an app password in the Bluesky app under Settings → Privacy and security → App passwords. It's only used to sign in: BiFrost keeps a sign-in token in your system's credential store, not the password.")
                    .line_height(1.5),
            );

        container(
            container(form)
                .padding(gap::XXL + gap::S)
                .style(theme::panel),
        )
        .center(Length::Fill)
        .into()
    }
}
