//! The New chat and Send in a chat dialogs.

use super::{Host, View};
use crate::components::{self as ui, Icon, ghost, icon, icon_button, status};
use crate::features::chat::Msg;
use crate::features::chat::people::NEW_CHAT_SEARCH;
use crate::features::chat::state::{MAX_MEMBERS, NewChat, Share};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, column, container, row, space, text};
use iced::{Alignment, Element, Length};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// Pick people for a new chat or group.
    pub fn new_chat(&self, n: &'a NewChat) -> Element<'a, M> {
        let lift = self.lift;
        let mut picked = Row::new().spacing(gap::XS);
        for a in &n.picked {
            picked = picked.push(
                button(
                    row![
                        self.avatar(a.avatar.as_deref(), 18.0),
                        text(a.name()).size(size::SMALL),
                        icon(Icon::Close, icon_size::XXS, Tone::Muted),
                    ]
                    .spacing(gap::XS)
                    .align_y(Alignment::Center),
                )
                .padding([gap::XS, gap::S])
                .style(theme::pill(true))
                .on_press(self.msg(Msg::NewUnpick(a.did.clone()))),
            );
        }
        let full = n.picked.len() >= MAX_MEMBERS - 1;
        let mut results = Column::new().spacing(gap::XXS);
        for a in n.results.iter().take(6) {
            let chosen = n.picked.iter().any(|p| p.did == a.did);
            results = results.push(
                button(
                    row![
                        self.avatar(a.avatar.as_deref(), 32.0),
                        column![
                            text(a.name()).size(size::BODY).font(theme::MEDIUM),
                            ui::faint(format!("@{}", a.handle)).size(size::SMALL),
                        ]
                        .spacing(gap::HAIR)
                        .width(Length::Fill),
                        if chosen {
                            Element::from(icon(Icon::Check, icon_size::M, Tone::Accent))
                        } else {
                            Element::from(space::horizontal().width(16))
                        },
                    ]
                    .spacing(gap::M)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(gap::S)
                .style(theme::row_button)
                .on_press_maybe((!chosen && !full).then(|| self.msg(Msg::NewPick(a.clone())))),
            );
        }
        let hint = match n.picked.len() {
            0 => "Pick one person for a chat, or several for a group.".to_string(),
            1 => "Add more people to make it a group chat.".to_string(),
            k if full => format!("{} people, the most a chat can have.", k + 1),
            k => format!("A group chat with {} people.", k + 1),
        };
        let label = match (n.starting, n.picked.len()) {
            (true, _) => "Starting…",
            (_, 0 | 1) => "Start chat",
            _ => "Start group chat",
        };
        let mut col = column![
            row![
                text("New chat").size(size::TITLE).font(theme::SEMIBOLD),
                space::horizontal(),
                icon_button(Icon::Close, self.msg(Msg::CloseNew)),
            ]
            .align_y(Alignment::Center),
            ui::field("Search for people", &n.query)
                .id(NEW_CHAT_SEARCH)
                .on_input(move |q| lift(Msg::NewQuery(q))),
        ]
        .spacing(gap::M);
        if !n.picked.is_empty() {
            col = col.push(picked.wrap().vertical_spacing(gap::XS));
        }
        col = col.push(results).push(ui::faint(hint).size(size::SMALL));
        if let Some(e) = &n.error {
            col = col.push(text(e).size(size::SMALL).style(theme::danger_text));
        }
        col = col.push(
            row![
                space::horizontal(),
                ghost("Cancel", self.msg(Msg::CloseNew)),
                ui::primary(label).on_press_maybe(
                    (!n.picked.is_empty() && !n.starting).then(|| self.msg(Msg::NewStart))
                ),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        );
        container(col)
            .padding(gap::XL)
            .width(theme::layout::DIALOG)
            .style(theme::dialog)
            .into()
    }

    /// Send a post in one of your chats.
    pub fn share(&self, s: &'a Share) -> Element<'a, M> {
        let lift = self.lift;
        let me = self.me;
        let mut chats = Column::new().spacing(gap::XXS);
        if self.chats.no_access {
            chats = chats.push(ui::muted(
                "BiFrost can't open your chats with this sign-in. Open Chats to see why.",
            ));
        } else if self.chats.accepted.ids.is_empty() {
            chats = chats.push(status(if self.chats.accepted.loaded {
                "No chats yet. Start one from Chats first."
            } else {
                "Loading your chats…"
            }));
        }
        for id in self.chats.accepted.ids.iter().take(8) {
            let Some(c) = self.chats.convos.get(id) else {
                continue;
            };
            let first = c.others(me).next();
            let action: Element<'a, M> = if s.sent.contains(id) {
                ui::faint("Sent").size(size::SMALL).into()
            } else if s.sending.as_deref() == Some(id.as_str()) {
                ui::faint("Sending…").size(size::SMALL).into()
            } else {
                ui::secondary("Send")
                    .on_press_maybe(
                        s.sending
                            .is_none()
                            .then(|| self.msg(Msg::ShareTo(id.clone()))),
                    )
                    .into()
            };
            chats = chats.push(
                row![
                    if c.is_group(me) {
                        self.stack_avatars(first, c.others(me).nth(1))
                    } else {
                        self.avatar(first.and_then(|a| a.avatar.as_deref()), 32.0)
                    },
                    text(c.title(me))
                        .size(size::BODY)
                        .wrapping(text::Wrapping::None)
                        .width(Length::Fill),
                    action,
                ]
                .spacing(gap::M)
                .padding([gap::XS, gap::S])
                .align_y(Alignment::Center),
            );
        }
        let mut col = column![
            row![
                text("Send in a chat")
                    .size(size::TITLE)
                    .font(theme::SEMIBOLD),
                space::horizontal(),
                icon_button(Icon::Close, self.msg(Msg::CloseShare)),
            ]
            .align_y(Alignment::Center),
            ui::field("Add a message (optional)", &s.text)
                .on_input(move |t| lift(Msg::ShareText(t))),
            chats,
        ]
        .spacing(gap::M);
        if let Some(e) = &s.error {
            col = col.push(text(e).size(size::SMALL).style(theme::danger_text));
        }
        col = col.push(
            row![
                space::horizontal(),
                ghost("Done", self.msg(Msg::CloseShare))
            ]
            .align_y(Alignment::Center),
        );
        container(col)
            .padding(gap::XL)
            .width(theme::layout::DIALOG)
            .style(theme::dialog)
            .into()
    }
}
