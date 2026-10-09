//! One chat: its messages, their menus and reactions, and the message field.

use super::{Host, View};
use crate::api::richtext::grapheme_len;
use crate::components::{self as ui, Icon, full_time, ghost, icon, popover, status};
use crate::features::chat::Msg;
use crate::features::chat::load::input_id;
use crate::features::chat::state::{MAX_MESSAGE, REACTIONS};
use crate::model::{ChatMessage, Convo, Embed, EmbeddedRecord};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, radius, size};
use iced::widget::{Column, Row, button, column, container, row, space, text};
use iced::{Alignment, Element, Length, Padding};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// One chat: its messages, a request's Accept and Decline, and the message field.
    pub fn chat(&self, id: &'a str) -> Element<'a, M> {
        if self.chats.no_access {
            return self.scroll("chat", self.no_access(), None);
        }
        let me = self.me;
        let convo = self.chats.convos.get(id);
        let group = convo.is_some_and(|c| c.is_group(me));
        let thread = self.chats.threads.get(id);

        let mut col = Column::new().spacing(gap::XXS).padding([gap::S, gap::L]);
        match thread {
            Some(t) if t.loading && !t.loaded => col = col.push(status("Loading…")),
            Some(t) if t.error.is_some() && t.messages.is_empty() => {
                col = col.push(self.retry(t.error.as_deref().unwrap_or_default()));
            }
            Some(t) => {
                if t.loading {
                    col = col.push(status("Loading older messages…"));
                } else if t.error.is_some() {
                    col = col.push(status(
                        "Couldn't load older messages. Refresh to try again.",
                    ));
                } else if t.cursor.is_none() && t.loaded {
                    col = col.push(status(match convo {
                        Some(c) => format!("The start of your chat with {}.", c.title(me)),
                        None => "The start of this chat.".into(),
                    }));
                }
                if t.loaded && t.messages.is_empty() {
                    col = col.push(status("Say hello."));
                }
                let mut prev: Option<&ChatMessage> = None;
                for (i, m) in t.messages.iter().enumerate() {
                    let next = t.messages.get(i + 1);
                    if prev.is_none_or(|p| day(&p.sent_at) != day(&m.sent_at)) {
                        col = col.push(
                            container(ui::faint(day_label(&m.sent_at)).size(size::CAPTION))
                                .padding([gap::M, 0.0])
                                .center_x(Length::Fill),
                        );
                    }
                    let starts_run = prev.is_none_or(|p| !p.same_run(m));
                    let ends_run = next.is_none_or(|n| !m.same_run(n));
                    col = col.push(self.message(id, convo, m, group, starts_run, ends_run));
                    prev = Some(m);
                }
            }
            None => col = col.push(status("Loading…")),
        }

        let lift = self.lift;
        let key = format!("chat:{id}");
        let cid = id.to_string();
        let messages =
            ui::scroll(container(ui::reading(col)).padding(Padding::ZERO.bottom(gap::M)))
                .anchor_bottom()
                .id(key)
                .on_scroll(move |v| lift(Msg::Scrolled(cid.clone(), v)));

        let mut bottom = Column::new().spacing(gap::S);
        if let Some(c) = convo.filter(|c| c.is_request()) {
            bottom = bottom.push(self.request(id, c, group));
        }
        let draft = thread.map_or("", |t| t.draft.as_str());
        let sending = thread.is_some_and(|t| t.sending);
        let len = grapheme_len(draft);
        let can_send = !sending && !draft.trim().is_empty() && len <= MAX_MESSAGE;
        let cid = id.to_string();
        let mut input = ui::field(
            if sending {
                "Sending…"
            } else {
                "Write a message"
            },
            draft,
        )
        .id(input_id(id))
        .padding([gap::S + 2.0, gap::M]);
        if !sending {
            input = input
                .on_input(move |v| lift(Msg::Draft(cid.clone(), v)))
                .on_submit(self.msg(Msg::Send(id.into())));
        }
        let mut compose = row![input].spacing(gap::S).align_y(Alignment::Center);
        if len > MAX_MESSAGE - 100 {
            let t = text(format!("{}", MAX_MESSAGE as i64 - len as i64)).size(size::CAPTION);
            compose = compose.push(if len > MAX_MESSAGE {
                t.style(theme::danger_text)
            } else {
                t.style(theme::faint)
            });
        }
        compose = compose.push(
            button(icon(
                Icon::Send,
                icon_size::L,
                if can_send {
                    Tone::OnAccent
                } else {
                    Tone::Faint
                },
            ))
            .padding(gap::S + 1.0)
            .style(theme::primary)
            .on_press_maybe(can_send.then(|| self.msg(Msg::Send(id.into())))),
        );
        bottom = bottom.push(compose);

        column![
            messages,
            ui::reading(container(bottom).padding(Padding::from([gap::S, gap::L]).bottom(gap::L))),
        ]
        .into()
    }

    /// "This is a chat request", with Accept, Decline, Block and Report.
    fn request(&self, id: &'a str, c: &'a Convo, group: bool) -> Element<'a, M> {
        let mut actions = row![
            ui::primary("Accept").on_press(self.msg(Msg::Accept(id.into()))),
            if self.chats.confirm_leave.as_deref() == Some(id) {
                ui::danger("Really decline?", self.msg(Msg::Leave(id.into())))
            } else {
                ghost("Decline", self.msg(Msg::Leave(id.into())))
            },
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center);
        if !group && let Some(other) = c.others(self.me).next() {
            actions = actions
                .push(ghost("Block", self.msg(Msg::Block(other.did.clone()))))
                .push(ghost("Report", self.msg(Msg::Report(other.did.clone()))));
        }
        ui::card(
            column![
                text("This is a chat request")
                    .size(size::BODY)
                    .font(theme::MEDIUM),
                ui::muted(
                    "You don't follow them. Accept to move it to your chats, or reply to accept it."
                ),
                actions,
            ]
            .spacing(gap::S),
        )
        .padding(gap::L)
        .into()
    }

    fn message(
        &self,
        convo_id: &'a str,
        convo: Option<&'a Convo>,
        m: &'a ChatMessage,
        group: bool,
        starts_run: bool,
        ends_run: bool,
    ) -> Element<'a, M> {
        let mine = m.sender.did == *self.me;
        let sender = convo.and_then(|c| c.members.iter().find(|a| a.did == m.sender.did));

        let mut bubble = Column::new().spacing(gap::S);
        if m.is_deleted() {
            bubble = bubble.push(ui::faint("Message deleted").size(size::BODY));
        } else {
            if !m.body.text.is_empty() {
                bubble = bubble.push(self.host.rich_text(&m.body));
            }
            match &m.embed {
                Some(Embed::Record {
                    record: EmbeddedRecord::Post(q),
                }) => bubble = bubble.push(self.host.quoted_post(q)),
                Some(_) => {
                    bubble =
                        bubble.push(ui::faint("A post that isn't available").size(size::SMALL));
                }
                None => {}
            }
        }
        let bubble = container(bubble)
            .padding([gap::S + 1.0, gap::M + 2.0])
            .max_width(460)
            .style(move |t: &iced::Theme| bubble_style(t, mine));

        let open = self.chats.menu.as_deref() == Some(m.id.as_str());
        let menu_button = button(icon(Icon::More, icon_size::S, Tone::Faint))
            .padding(gap::XS)
            .style(theme::ghost)
            .on_press(self.msg(Msg::Menu(if open { None } else { Some(m.id.clone()) })));
        let menu: Element<'a, M> = if m.is_deleted() {
            space::horizontal().width(22).into()
        } else {
            popover(
                menu_button,
                open.then(|| self.menu(convo_id, m)),
                self.msg(Msg::Menu(None)),
            )
            .into()
        };

        let mut body = Column::new().spacing(gap::XXS);
        if group && !mine && starts_run {
            body = body.push(
                container(ui::faint(sender.map_or("Someone", |a| a.name())).size(size::CAPTION))
                    .padding(Padding::ZERO.left(gap::M)),
            );
        }
        let line: Row<'a, M> = if mine {
            row![space::horizontal(), menu, bubble]
        } else {
            row![bubble, menu, space::horizontal()]
        };
        body = body.push(line.spacing(gap::XS).align_y(Alignment::Center));

        if !m.reactions.is_empty() {
            body = body.push(self.reactions(convo_id, m, mine));
        }
        if ends_run {
            let t = container(ui::tip(
                ui::faint(time_of_day(&m.sent_at)).size(size::CAPTION),
                full_time(&m.sent_at),
            ))
            .padding(Padding::ZERO.left(gap::M).right(gap::M + 26.0));
            body = body.push(if mine {
                container(t).align_right(Length::Fill)
            } else {
                container(t)
            });
        }

        let mut out = Row::new().spacing(gap::S).align_y(Alignment::End);
        if group && !mine {
            out = out.push(if ends_run {
                ui::hand(self.avatar(sender.and_then(|a| a.avatar.as_deref()), 28.0))
                    .on_press(self.msg(Msg::OpenProfile(m.sender.did.clone())))
                    .into()
            } else {
                Element::from(space::horizontal().width(28))
            });
        }
        out = out.push(body.width(Length::Fill));
        let gap_above = if starts_run { gap::S } else { 0.0 };
        container(out).padding(Padding::ZERO.top(gap_above)).into()
    }

    fn reactions(&self, convo_id: &'a str, m: &'a ChatMessage, mine: bool) -> Element<'a, M> {
        let mut counts: Vec<(&str, usize, bool)> = Vec::new();
        for r in &m.reactions {
            let by_me = r.sender.did == *self.me;
            match counts.iter_mut().find(|(v, ..)| *v == r.value) {
                Some(c) => {
                    c.1 += 1;
                    c.2 |= by_me;
                }
                None => counts.push((r.value.as_str(), 1, by_me)),
            }
        }
        let mut r = Row::new().spacing(gap::XS);
        if mine {
            r = r.push(space::horizontal());
        }
        for (value, n, by_me) in counts {
            let mut label = row![text(value).size(size::EMOJI)]
                .spacing(gap::XS)
                .align_y(Alignment::Center);
            if n > 1 {
                label = label.push(text(n.to_string()).size(size::SMALL));
            }
            r = r.push(
                button(label)
                    .padding([gap::HAIR, gap::S])
                    .style(theme::pill(by_me))
                    .on_press(self.msg(Msg::React(
                        convo_id.into(),
                        m.id.clone(),
                        value.into(),
                        !by_me,
                    ))),
            );
        }
        container(r)
            .padding(Padding::ZERO.left(gap::S).right(gap::S + 26.0))
            .into()
    }

    /// A message's menu: quick reactions, Copy text and Delete for you.
    fn menu(&self, convo_id: &'a str, m: &'a ChatMessage) -> Element<'a, M> {
        let mut emoji = Row::new().spacing(gap::XXS);
        for value in REACTIONS {
            let by_me = m
                .reactions
                .iter()
                .any(|r| r.value == value && r.sender.did == *self.me);
            emoji = emoji.push(
                button(text(value).size(size::EMOJI + 4.0))
                    .padding([gap::XS, gap::S - 2.0])
                    .style(theme::pill(by_me))
                    .on_press(self.msg(Msg::React(
                        convo_id.into(),
                        m.id.clone(),
                        value.into(),
                        !by_me,
                    ))),
            );
        }
        let item = |i: Icon, label: &'a str, tint: Tone, msg: M| {
            button(
                row![
                    icon(i, icon_size::SM, tint),
                    text(label).size(size::SMALL).style(theme::tone(tint))
                ]
                .spacing(gap::S)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([gap::S, gap::M - 2.0])
            .style(theme::row_button)
            .on_press(msg)
        };
        container(
            column![
                container(emoji).padding(gap::XS),
                item(
                    Icon::Copy,
                    "Copy text",
                    Tone::Text,
                    self.msg(Msg::CopyText(m.body.text.clone()))
                ),
                item(
                    Icon::Trash,
                    "Delete for you",
                    Tone::Danger,
                    self.msg(Msg::Delete(convo_id.into(), m.id.clone()))
                ),
            ]
            .spacing(gap::XXS)
            .width(250),
        )
        .padding(gap::XS)
        .style(theme::overlay)
        .into()
    }
}

fn bubble_style(t: &iced::Theme, mine: bool) -> container::Style {
    let p = theme::palette(t);
    container::Style {
        background: Some(if mine { p.accent_soft } else { p.raised }.into()),
        border: iced::Border::default().rounded(radius::L),
        ..Default::default()
    }
}

fn local(iso: &str) -> Option<chrono::DateTime<chrono::Local>> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|t| t.with_timezone(&chrono::Local))
}

fn day(iso: &str) -> Option<chrono::NaiveDate> {
    local(iso).map(|t| t.date_naive())
}

/// "Today", "Yesterday", a weekday this week, or the full date.
fn day_label(iso: &str) -> String {
    let Some(t) = local(iso) else {
        return String::new();
    };
    let today = chrono::Local::now().date_naive();
    match (today - t.date_naive()).num_days() {
        0 => "Today".into(),
        1 => "Yesterday".into(),
        2..7 => t.format("%A").to_string(),
        _ => t.format("%A, %b %-d, %Y").to_string(),
    }
}

fn time_of_day(iso: &str) -> String {
    local(iso)
        .map(|t| t.format("%-I:%M %p").to_string())
        .unwrap_or_default()
}
