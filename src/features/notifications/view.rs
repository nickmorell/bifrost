//! The notifications list. Replies, mentions and quotes show the post itself, which the
//! app draws, so the list builds the app's own message type `M` through [`View::lift`].

use super::group::Filter;
use super::message::Msg;
use super::state::Notifications;
use crate::components::{
    self as ui, Icon, Window, compact, full_time, icon, lazy_list, pill, relative_time, row_key,
    status,
};
use crate::features::Pictures;
use crate::model::{Author, Notification, PostView};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, column, container, row, text};
use iced::{Alignment, Element, Length, Padding};

/// Height assumed for a row not drawn yet.
const ESTIMATED_ROW: f32 = 110.0;
/// Avatars shown in a grouped notification before "+N".
const MAX_FACES: usize = 6;

/// What the notifications list needs from the app around it.
pub trait Host<M>: Pictures {
    /// A full post with its actions.
    fn post<'a>(&'a self, post: &'a PostView) -> Element<'a, M>;
}

/// The notifications list.
pub struct View<'a, H, M> {
    pub notifications: &'a Notifications,
    pub host: &'a H,
    /// The part of the list on screen; only rows near it are drawn.
    pub window: Window,
    /// Turns the feature's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    fn msg(&self, m: Msg) -> M {
        (self.lift)(m)
    }

    fn avatar(&self, url: Option<&str>, size: f32) -> Element<'a, M> {
        ui::avatar(self.host.picture(url), size)
    }

    /// The notifications with their filter, unscrolled, for the screen or a deck column.
    pub fn body(&self) -> Element<'a, M> {
        let n = self.notifications;
        let filters = row![
            pill(
                "All",
                n.filter == Filter::All,
                self.msg(Msg::Filter(Filter::All))
            ),
            pill(
                "Mentions",
                n.filter == Filter::Mentions,
                self.msg(Msg::Filter(Filter::Mentions))
            ),
        ]
        .spacing(gap::XS);
        let mut col = Column::new()
            .spacing(gap::XXS)
            .push(container(filters).padding([gap::XS, gap::L + gap::XS]));
        let rows = n.rows.iter().filter_map(|g| {
            let members: Vec<&'a Notification> =
                g.members.iter().filter_map(|&i| n.items.get(i)).collect();
            Some((row_key(members.first()?.uri.as_str()), members))
        });
        let lift = self.lift;
        col = col.push(lazy_list(
            rows,
            &n.heights,
            ESTIMATED_ROW,
            self.window,
            // Rows sit `gap::XXS` apart, inside each row so it counts in its height.
            |i, members| {
                let top = if i == 0 { 0.0 } else { gap::XXS };
                container(self.row(&members))
                    .padding(Padding::ZERO.top(top))
                    .into()
            },
            move |row, height| lift(Msg::Measured(row, height)),
        ));
        if n.loading {
            col = col.push(status("Loading…"));
        } else if n.loaded && n.rows.is_empty() {
            col = col.push(status(match n.filter {
                Filter::All => "No notifications yet.",
                Filter::Mentions => "No replies, mentions or quotes yet.",
            }));
        }
        col.into()
    }

    /// One row of notifications: a single reply or mention, or everyone who liked,
    /// reposted or followed together.
    fn row(&self, members: &[&'a Notification]) -> Element<'a, M> {
        let n = members[0];
        let subject = n
            .subject_uri()
            .and_then(|u| self.notifications.subjects.get(u));
        if matches!(n.reason.as_str(), "reply" | "mention" | "quote")
            && let Some(p) = subject
        {
            let (i, label) = match n.reason.as_str() {
                "reply" => (Icon::Reply, "replied to you"),
                "mention" => (Icon::Bell, "mentioned you"),
                _ => (Icon::Quote, "quoted your post"),
            };
            return column![
                container(
                    row![
                        icon(i, icon_size::XS, Tone::Accent),
                        ui::faint(format!("{} {label}", n.author.name())),
                    ]
                    .spacing(gap::XS + 2.0)
                    .align_y(Alignment::Center)
                )
                .padding(Padding::from([0.0, gap::XL]).top(gap::M)),
                self.host.post(p),
            ]
            .into();
        }

        let others = members.len() - 1;
        let expanded = others > 0 && self.notifications.expanded.contains(n.uri.as_str());
        let (i, tint, what) = match n.reason.as_str() {
            "like" => (Icon::HeartFilled, Tone::Rose, "liked your post"),
            "repost" => (Icon::Repost, Tone::Accent, "reposted your post"),
            "follow" => (Icon::Follow, Tone::Accent, "followed you"),
            "like-via-repost" => (Icon::HeartFilled, Tone::Rose, "liked your repost"),
            "repost-via-repost" => (Icon::Repost, Tone::Accent, "reposted your repost"),
            "starterpack-joined" => (Icon::Follow, Tone::Accent, "joined via your starter pack"),
            "verified" => (Icon::Check, Tone::Accent, "verified your account"),
            "unverified" => (Icon::Close, Tone::Muted, "removed your verification"),
            "subscribed-post" => (Icon::Bell, Tone::Accent, "posted"),
            _ => (Icon::Bell, Tone::Muted, "interacted with you"),
        };
        let profile = |a: &Author| self.msg(Msg::OpenProfile(a.did.clone()));
        let mut faces = Row::new().spacing(gap::XS);
        for m in members.iter().take(MAX_FACES) {
            faces = faces.push(
                ui::hand(self.avatar(m.author.avatar.as_deref(), 32.0))
                    .on_press(profile(&m.author)),
            );
        }
        if others + 1 > MAX_FACES {
            faces = faces.push(ui::faint(format!("+{}", others + 1 - MAX_FACES)));
        }
        let mut line = row![
            button(text(n.author.name()).size(size::BODY).font(theme::SEMIBOLD))
                .padding(0)
                .style(theme::bare)
                .on_press(profile(&n.author)),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center);
        if others > 0 {
            let label = match others {
                1 => "and 1 other".to_string(),
                n => format!("and {} others", compact(n as u64)),
            };
            line = line.push(
                button(text(label).size(size::BODY).font(theme::MEDIUM))
                    .padding(0)
                    .style(theme::bare)
                    .on_press(self.msg(Msg::Expand(n.uri.to_string()))),
            );
        }
        line = line.push(ui::muted(what).size(size::BODY));
        let follow_back = |a: &'a Author| -> Option<Element<'a, M>> {
            let v = &a.viewer;
            (v.following.is_none() && v.blocking.is_none() && !v.blocked_by).then(|| {
                ui::secondary("Follow back")
                    .on_press(self.msg(Msg::FollowBack(a.did.clone())))
                    .into()
            })
        };
        let mut top = row![
            column![
                faces,
                line.wrap(),
                ui::tip(
                    ui::faint(relative_time(&n.indexed_at)).size(size::CAPTION),
                    full_time(&n.indexed_at)
                ),
            ]
            .spacing(gap::XS)
            .width(Length::Fill),
        ]
        .spacing(gap::M)
        .align_y(Alignment::Center);
        if n.reason == "follow"
            && others == 0
            && let Some(b) = follow_back(&n.author)
        {
            top = top.push(b);
        }
        let mut col = column![
            row![
                container(icon(i, icon_size::L, tint)).padding(Padding::ZERO.top(gap::XS + 2.0)),
                top
            ]
            .spacing(gap::M)
        ]
        .spacing(gap::S);
        let indent = 18.0 + gap::M;
        if expanded {
            let mut list = Column::new().spacing(gap::XXS);
            for m in members {
                let mut r = row![
                    button(
                        row![
                            self.avatar(m.author.avatar.as_deref(), 24.0),
                            text(m.author.name()).size(size::SMALL).font(theme::MEDIUM),
                            ui::faint(format!("@{}", m.author.handle)),
                        ]
                        .spacing(gap::S)
                        .align_y(Alignment::Center),
                    )
                    .padding([gap::XS, gap::S])
                    .style(theme::row_button)
                    .width(Length::Fill)
                    .on_press(profile(&m.author)),
                ]
                .align_y(Alignment::Center);
                if n.reason == "follow"
                    && let Some(b) = follow_back(&m.author)
                {
                    r = r.push(b);
                }
                list = list.push(r);
            }
            col = col.push(container(list).padding(Padding::ZERO.left(indent - gap::S)));
        }
        if let Some(p) = subject {
            col = col.push(
                ui::hand(
                    container(ui::muted(ui::truncate_lines(&p.record.text, 3)))
                        .padding(Padding::ZERO.left(indent)),
                )
                .on_press(self.msg(Msg::OpenThread(p.uri.clone()))),
            );
        }
        let card = container(col).padding([gap::M, gap::M]).width(Length::Fill);
        let card = if members.iter().all(|m| m.is_read) {
            card
        } else {
            card.style(theme::unread)
        };
        container(card).padding([0.0, gap::M]).into()
    }
}
