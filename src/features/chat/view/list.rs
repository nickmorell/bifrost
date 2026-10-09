//! The chat list: your chats or your requests.

use super::{Host, View};
use crate::components::{self as ui, Icon, icon, pill, relative_time, status};
use crate::features::chat::Msg;
use crate::model::Convo;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, button, column, container, row, space, text};
use iced::{Alignment, Element, Length};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// Your chats, or the requests, with New chat.
    pub fn list(&self) -> Element<'a, M> {
        let mut col = Column::new().spacing(gap::XS).padding([0.0, gap::L]);
        if self.chats.no_access {
            return self.scroll("chats", col.push(self.no_access()), None);
        }
        let requests = self.chats.showing_requests;
        let request_count = self.chats.requests.ids.len();
        let label = match (self.chats.requests.loaded, request_count) {
            (true, n) if n > 0 => format!("Requests · {n}"),
            _ => "Requests".to_string(),
        };
        col = col.push(
            container(
                row![
                    pill("Chats", !requests, self.msg(Msg::Tab(false))),
                    pill(label, requests, self.msg(Msg::Tab(true))),
                    space::horizontal(),
                    ui::primary_icon(Icon::Plus, "New chat")
                        .padding([gap::S - 1.0, gap::M])
                        .on_press(self.msg(Msg::NewChat)),
                ]
                .spacing(gap::XS)
                .align_y(Alignment::Center),
            )
            .padding([gap::S, gap::S]),
        );
        let list = if requests {
            &self.chats.requests
        } else {
            &self.chats.accepted
        };
        if let Some(e) = &list.error
            && list.ids.is_empty()
        {
            return self.scroll("chats", col.push(self.retry(e.as_str())), None);
        }
        if list.ids.is_empty() {
            col = col.push(if !list.loaded {
                status("Loading…")
            } else if requests {
                status(
                    "No chat requests. When someone you don't follow messages you, it waits here.",
                )
            } else {
                status(
                    "No chats yet. Start one with New chat, or with Message on someone's profile.",
                )
            });
        }
        for id in &list.ids {
            if let Some(c) = self.chats.convos.get(id) {
                col = col.push(self.convo_row(c));
            }
        }
        if list.loading && !list.ids.is_empty() {
            col = col.push(status("Loading…"));
        }
        let more = list.cursor.is_some().then(|| self.msg(Msg::MoreConvos));
        self.scroll("chats", col, more)
    }

    fn convo_row(&self, c: &'a Convo) -> Element<'a, M> {
        let me = self.me;
        let mut others = c.others(me);
        let first = others.next();
        let pic: Element<'a, M> = if c.is_group(me) {
            let second = others.next();
            self.stack_avatars(first, second)
        } else {
            self.avatar(first.and_then(|a| a.avatar.as_deref()), 42.0)
        };
        let unread = c.unread_count > 0;
        let mut title = row![
            text(c.title(me))
                .size(size::BODY)
                .font(if unread {
                    theme::SEMIBOLD
                } else {
                    theme::MEDIUM
                })
                .wrapping(text::Wrapping::None)
                .width(Length::Fill),
        ]
        .spacing(gap::XS)
        .align_y(Alignment::Center);
        if c.muted {
            title = title.push(icon(Icon::BellOff, icon_size::XS, Tone::Faint));
        }
        if let Some(m) = &c.last_message {
            title = title.push(ui::faint(relative_time(&m.sent_at)).size(size::CAPTION));
        }
        let handle = match (c.is_group(me), first) {
            (false, Some(a)) => format!("@{}", a.handle),
            _ => format!("{} people", c.members.len()),
        };
        let preview = match &c.last_message {
            Some(m) if m.sender.did == *me => format!("You: {}", m.preview()),
            Some(m) if c.is_group(me) => match c.members.iter().find(|a| a.did == m.sender.did) {
                Some(a) => format!("{}: {}", a.name(), m.preview()),
                None => m.preview(),
            },
            Some(m) => m.preview(),
            None => handle,
        };
        let mut line = row![
            text(preview)
                .size(size::SMALL)
                .style(if unread { theme::muted } else { theme::faint })
                .wrapping(text::Wrapping::None)
                .width(Length::Fill)
        ]
        .align_y(Alignment::Center);
        if unread {
            line = line.push(ui::count_badge(c.unread_count));
        }
        button(
            row![
                pic,
                column![title, line].spacing(gap::XXS).width(Length::Fill),
            ]
            .spacing(gap::M)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([gap::S + 2.0, gap::M])
        .style(theme::row_button)
        .on_press(self.msg(Msg::Open(c.id.clone())))
        .into()
    }
}
