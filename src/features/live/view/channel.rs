//! A streamer's page: their stream (or that they are offline), who they are, their
//! past streams, and the chat beside or below.

use super::board::title_of;
use super::{Host, View, heading, viewers};
use crate::components::{self as ui, Icon, ghost, video_player::live_mark};
use crate::features::live::state::Channel;
use crate::features::live::{Msg, player_key};
use crate::model::{Did, PastStream};
use crate::theme::icon_size;
use crate::theme::{self, gap, size};
use iced::widget::{Column, Row, column, container, responsive, row, space, stack, text};
use iced::{Alignment, Element, Length, Size};

/// Windows at least this wide show the chat beside the stream rather than below.
const WIDE: f32 = 900.0;
/// How wide the chat is beside the stream.
const CHAT_W: f32 = 340.0;
/// The widest the stream is drawn.
const MAX_PLAYER_W: f32 = 1280.0;

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// The page of the streamer opened as `actor` (a handle or DID).
    pub fn channel(&self, actor: &'a str) -> Element<'a, M> {
        let Some(c) = self.live.channels.get(actor) else {
            return ui::status("Loading…");
        };
        let Some(did) = c.did() else {
            return match &c.error {
                Some(e) => self.retry(e.clone(), Msg::Refresh(actor.into())),
                None => ui::status("Loading…"),
            };
        };
        let v = self.copy();
        responsive(move |space: Size| {
            let wide = space.width >= WIDE;
            let main_w = if wide {
                space.width - CHAT_W - gap::L
            } else {
                space.width
            };
            let main_h = if wide {
                space.height
            } else {
                space.height * 0.6
            };
            let main = ui::scroll(v.stage(actor, c, did, main_w, main_h))
                .width(Length::Fill)
                .height(Length::Fill);
            let chat = v.chat(actor, c);
            if wide {
                row![main, container(chat).width(CHAT_W).height(Length::Fill)]
                    .spacing(gap::L)
                    .padding(iced::Padding::ZERO.right(gap::L).bottom(gap::L))
                    .into()
            } else {
                column![
                    container(main).height(Length::FillPortion(3)),
                    container(chat)
                        .height(Length::FillPortion(2))
                        .padding([0.0, gap::L]),
                ]
                .spacing(gap::S)
                .padding(iced::Padding::ZERO.bottom(gap::L))
                .into()
            }
        })
        .into()
    }

    /// The stream, who is streaming, and their past streams, for a space `w` by `h`.
    fn stage(
        &self,
        actor: &'a str,
        c: &'a Channel,
        did: &'a Did,
        w: f32,
        h: f32,
    ) -> Element<'a, M> {
        let live = c.is_live(&self.now);
        let key = player_key(did);
        let mut pw = (w - 2.0 * gap::XL).clamp(160.0, MAX_PLAYER_W);
        let mut ph = pw * 9.0 / 16.0;
        let tallest = (h * 0.7).max(120.0);
        if ph > tallest {
            ph = tallest;
            pw = ph * 16.0 / 9.0;
        }
        let thumb = c.latest.as_ref().and_then(|s| s.thumb.as_deref());
        let screen = if live || self.host.playing(&key) {
            self.host
                .player(&key, thumb, self.msg(Msg::Watch(actor.into())), pw, ph)
        } else {
            self.offline(c, pw, ph)
        };

        let mut col = Column::new()
            .spacing(gap::L)
            .padding([gap::L, gap::XL])
            .push(container(screen).center_x(Length::Fill))
            .push(self.about(actor, c, did, live))
            .push(self.past(actor, c));
        if let Some(e) = &c.error {
            col = col.push(ui::faint(format!("Couldn't refresh this page: {e}")));
        }
        col.width(Length::Fill).into()
    }

    /// Their latest stream's still, dimmed, with "Offline" over it.
    fn offline(&self, c: &'a Channel, w: f32, h: f32) -> Element<'a, M> {
        let when = c
            .latest
            .as_ref()
            .and_then(|s| s.ended.as_deref().or(Some(&s.started)))
            .map(ui::relative_time)
            .filter(|t| !t.is_empty());
        let mut words = column![
            text("Offline")
                .size(size::TITLE)
                .font(theme::SEMIBOLD)
                .style(theme::on_media)
        ]
        .spacing(gap::XS)
        .align_x(Alignment::Center);
        words = words.push(
            text(match when {
                Some(t) => format!("Last streamed {t} ago"),
                None => "Hasn't streamed yet".into(),
            })
            .size(size::SMALL)
            .style(theme::on_media),
        );
        stack![
            self.still(c.latest.as_ref().and_then(|s| s.thumb.as_deref()), w, h),
            container(words).center_x(w).center_y(h).style(theme::scrim),
        ]
        .into()
    }

    /// Who is streaming and what: their picture, name, the stream's title and buttons.
    fn about(&self, actor: &'a str, c: &'a Channel, did: &'a Did, live: bool) -> Element<'a, M> {
        let Some(p) = &c.profile else {
            return space::vertical().height(0).into();
        };
        let pic = self.host.picture(p.avatar.as_deref());
        let avatar = if live {
            ui::live_avatar(pic, 52.0)
        } else {
            ui::avatar(pic, 52.0)
        };
        let mut who = column![
            text(p.name()).size(size::TITLE).font(theme::SEMIBOLD),
            ui::faint(format!("@{}", p.handle)).size(size::SMALL),
        ]
        .spacing(gap::XXS)
        .width(Length::Fill);
        if let Some(s) = &c.latest {
            who = who.push(text(title_of(s)).size(size::BODY).font(theme::MEDIUM));
            let mut facts = Row::new().spacing(gap::S).align_y(Alignment::Center);
            if live {
                facts = facts.push(live_mark());
                if let Some(n) = c.viewers.or(s.viewers) {
                    facts = facts.push(viewers(n));
                }
            }
            let mut words: Vec<String> = Vec::new();
            words.extend(s.activity.clone());
            let started = ui::relative_time(&s.started);
            if live && !started.is_empty() {
                words.push(format!("started {started} ago"));
            }
            if !words.is_empty() {
                facts = facts.push(ui::faint(words.join(" · ")).size(size::SMALL));
            }
            who = who.push(facts);
        }
        let actions = row![
            ghost("Profile", self.msg(Msg::OpenProfile(did.clone()))),
            ui::icon_action(
                Icon::External,
                "Open on stream.place",
                self.msg(Msg::OpenWeb(actor.into()))
            ),
        ]
        .spacing(gap::XS)
        .align_y(Alignment::Center);
        row![avatar, who, actions]
            .spacing(gap::M)
            .align_y(Alignment::Start)
            .into()
    }

    /// Their past streams, each with a button to watch it.
    fn past(&self, actor: &'a str, c: &'a Channel) -> Element<'a, M> {
        let mut col = Column::new().spacing(gap::XS).push(heading("Past streams"));
        if c.past.is_empty() {
            col = col.push(ui::faint(if c.past_loading {
                "Loading…"
            } else {
                "No past streams kept."
            }));
        }
        for p in &c.past {
            col = col.push(self.past_row(actor, p));
        }
        if c.past_cursor.is_some() {
            col = col.push(if c.past_loading {
                ui::faint("Loading…").into()
            } else {
                Element::from(ghost("Show more", self.msg(Msg::MorePast(actor.into()))))
            });
        }
        col.into()
    }

    fn past_row(&self, actor: &'a str, p: &'a PastStream) -> Element<'a, M> {
        let mut facts = vec![ui::relative_time(&p.created)];
        facts.extend(p.seconds.map(length));
        let title = if p.title.trim().is_empty() {
            "Untitled stream"
        } else {
            p.title.as_str()
        };
        ui::hand(
            container(
                row![
                    ui::icon(Icon::Play, icon_size::M, theme::Tone::Accent),
                    text(title).size(size::BODY).width(Length::Fill),
                    ui::faint(facts.join(" · ")).size(size::SMALL),
                ]
                .spacing(gap::M)
                .align_y(Alignment::Center),
            )
            .padding([gap::S, gap::M])
            .style(theme::raised),
        )
        .on_press(self.msg(Msg::WatchPast(actor.into(), p.uri.clone())))
        .into()
    }
}

/// A stream's length: "12:34", or "1:02:03" for an hour or more.
fn length(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lengths_read_like_clocks() {
        assert_eq!(super::length(754), "12:34");
        assert_eq!(super::length(3723), "1:02:03");
    }
}
