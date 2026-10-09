//! A stream's chat: the pinned message, messages with the newest at the bottom, and
//! the field to write in.

use super::{Host, View};
use crate::api::richtext::grapheme_len;
use crate::components::{self as ui, Icon, icon};
use crate::features::live::state::Channel;
use crate::features::live::{MAX_MESSAGE, Msg};
use crate::model::LiveChat;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, button, column, container, row, text};
use iced::{Alignment, Element, Length, Padding};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// The chat of the streamer opened as `actor`.
    pub(super) fn chat(&self, actor: &'a str, c: &'a Channel) -> Element<'a, M> {
        let state = if c.connected {
            ui::faint("Connected").size(size::CAPTION)
        } else {
            ui::faint("Connecting…").size(size::CAPTION)
        };
        let top = row![
            text("Chat")
                .size(size::BODY)
                .font(theme::SEMIBOLD)
                .width(Length::Fill),
            state
        ]
        .align_y(Alignment::Center);

        let mut messages = Column::new().spacing(gap::S).padding([gap::S, 0.0]);
        if c.chat.is_empty() {
            messages = messages.push(ui::faint(if c.connected {
                "No messages yet. Say hello."
            } else {
                "The chat shows here once it connects."
            }));
        }
        for m in &c.chat {
            messages = messages.push(self.line(m));
        }
        let scroll = ui::scroll(messages)
            .anchor_bottom()
            .height(Length::Fill)
            .width(Length::Fill);

        let mut col = Column::new().spacing(gap::S).push(top);
        if let Some(p) = &c.pinned {
            col = col.push(
                container(
                    column![
                        row![
                            icon(Icon::Pin, icon_size::XXS, Tone::Accent),
                            ui::faint("Pinned").size(size::CAPTION)
                        ]
                        .spacing(gap::XS)
                        .align_y(Alignment::Center),
                        self.line(p),
                    ]
                    .spacing(gap::XS),
                )
                .padding(gap::S)
                .width(Length::Fill)
                .style(theme::raised),
            );
        }
        col = col
            .push(ui::divider_flush())
            .push(scroll)
            .push(self.compose(actor, c));
        ui::outlined(col.height(Length::Fill))
            .padding(gap::M)
            .height(Length::Fill)
            .into()
    }

    /// One message: their picture, their name in the colour they picked, and the text.
    fn line(&self, m: &'a LiveChat) -> Element<'a, M> {
        let mut name = text(m.author.name())
            .size(size::SMALL)
            .font(theme::SEMIBOLD);
        if let Some(rgb) = m.color {
            name = name.style(theme::picked(rgb));
        }
        let mut body = Column::new().spacing(gap::XXS).width(Length::Fill).push(
            row![
                ui::hand(name).on_press(self.msg(Msg::OpenProfile(m.author.did.clone()))),
                ui::tip(
                    ui::faint(ui::relative_time(&m.body.created_at)).size(size::CAPTION),
                    ui::full_time(&m.body.created_at),
                ),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        );
        if let Some((who, said)) = &m.reply_to {
            body = body.push(
                ui::faint(format!("Replying to {who}: {}", ui::truncate(said, 60)))
                    .size(size::CAPTION),
            );
        }
        body = body.push(self.host.rich_text(&m.body));
        row![self.avatar(m.author.avatar.as_deref(), 24.0), body]
            .spacing(gap::S)
            .align_y(Alignment::Start)
            .into()
    }

    /// The field to write a message in, and Send.
    fn compose(&self, actor: &'a str, c: &'a Channel) -> Element<'a, M> {
        let lift = self.lift;
        let len = grapheme_len(&c.draft);
        let can_send = !c.sending && !c.draft.trim().is_empty() && len <= MAX_MESSAGE;
        let a = actor.to_string();
        let mut input = ui::field(
            if c.sending {
                "Sending…"
            } else {
                "Send a message"
            },
            &c.draft,
        )
        .padding([gap::S, gap::M]);
        if !c.sending {
            input = input
                .on_input(move |v| lift(Msg::Draft(a.clone(), v)))
                .on_submit(self.msg(Msg::Send(actor.into())));
        }
        let send = button(icon(
            Icon::Send,
            icon_size::M,
            if can_send {
                Tone::OnAccent
            } else {
                Tone::Faint
            },
        ))
        .padding(gap::S)
        .style(theme::primary)
        .on_press_maybe(can_send.then(|| self.msg(Msg::Send(actor.into()))));
        let mut col =
            column![row![input, send].spacing(gap::S).align_y(Alignment::Center)].spacing(gap::XS);
        if len > MAX_MESSAGE - 50 {
            let t = text(format!("{} left", MAX_MESSAGE as i64 - len as i64)).size(size::CAPTION);
            col = col.push(if len > MAX_MESSAGE {
                t.style(theme::danger_text)
            } else {
                t.style(theme::faint)
            });
        }
        if let Some(e) = &c.send_error {
            col = col.push(
                text(format!("Couldn't send: {e}"))
                    .size(size::CAPTION)
                    .style(theme::danger_text),
            );
        }
        container(col).padding(Padding::ZERO.top(gap::XS)).into()
    }
}
