//! The top of a profile: banner, picture, name, counts, bio and actions.

use super::{Host, View};
use crate::components::{self as ui, Icon, compact, ghost, icon_button};
use crate::features::profile::message::Msg;
use crate::model::{PeopleKind, Profile, ProfileLink};
use crate::theme::{self, gap, size};
use iced::widget::{Column, Row, button, column, container, row, space, text};
use iced::{Alignment, Element, Length};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    pub(super) fn header(&self, p: &'a Profile) -> Element<'a, M> {
        let mine = p.did == self.me;
        let people = |kind| self.msg(Msg::People(p.did.to_string(), kind));
        let stat = |n: u64, label: &'static str, open: Option<M>| -> Element<'a, M> {
            let content = row![
                text(compact(n)).size(size::SMALL).font(theme::SEMIBOLD),
                ui::faint(label),
            ]
            .spacing(gap::XS + 2.0);
            match open {
                Some(msg) => button(content)
                    .padding([gap::XXS, gap::XS])
                    .style(theme::ghost)
                    .on_press(msg)
                    .into(),
                None => container(content).padding([gap::XXS, gap::XS]).into(),
            }
        };

        let mut handle_line = row![ui::muted(format!("@{}", p.handle))]
            .spacing(gap::S)
            .align_y(Alignment::Center);
        if let Some(pronouns) = p.pronouns() {
            handle_line = handle_line.push(ui::faint(pronouns));
        }
        let now = chrono::Utc::now().to_rfc3339();
        let live = p.status.as_ref().and_then(|s| s.live_now(&now));
        if let Some(l) = live {
            handle_line = handle_line.push(ghost(
                format!("Live on {}", l.service),
                self.msg(Msg::OpenUrl(l.url.to_string())),
            ));
        }
        let pic = self.host.picture(p.avatar.as_deref());
        let avatar: Element<'a, M> = match live {
            Some(l) => ui::hand(ui::live_avatar(pic, 76.0))
                .on_press(self.msg(Msg::OpenUrl(l.url.to_string())))
                .into(),
            None => ui::avatar(pic, 76.0),
        };
        if p.viewer.followed_by.is_some() && !mine {
            handle_line = handle_line.push(
                container(text("Follows you").size(size::CAPTION))
                    .padding([1.0, 8.0])
                    .style(theme::raised),
            );
        }

        let mut col = Column::new().spacing(gap::L).padding([gap::M, gap::XL]);
        if let Some(banner) = p.banner.as_deref() {
            col = col.push(match self.host.picture(Some(banner)) {
                Some(ui::Picture::Loaded(handle)) => Element::from(
                    iced::widget::image(handle)
                        .width(Length::Fill)
                        .height(180)
                        .content_fit(iced::ContentFit::Cover)
                        .border_radius(theme::radius::L),
                ),
                _ => container(space::vertical().height(180))
                    .width(Length::Fill)
                    .style(theme::placeholder(theme::radius::L))
                    .into(),
            });
        }
        col = col
            .push(
                row![
                    avatar,
                    column![self.name_line(p), handle_line].spacing(gap::XS),
                ]
                .spacing(gap::L + 2.0)
                .align_y(Alignment::Center),
            )
            .push(
                row![
                    stat(
                        p.followers_count,
                        "followers",
                        Some(people(PeopleKind::Followers))
                    ),
                    stat(
                        p.follows_count,
                        "following",
                        Some(people(PeopleKind::Following))
                    ),
                    stat(p.posts_count, "posts", None),
                ]
                .spacing(gap::M),
            );
        if let Some(d) = p.description.as_deref().filter(|d| !d.trim().is_empty()) {
            col = col.push(self.host.linked_text(d.trim()));
        }
        let links = p.shown_links();
        if !links.is_empty() {
            col = col.push(self.links(links).wrap());
        }
        col.push(self.actions(p).wrap()).into()
    }

    /// Their name, with its verification check.
    fn name_line(&self, p: &'a Profile) -> Row<'a, M> {
        let mut line = row![text(p.name()).size(size::TITLE + 2.0).font(theme::BOLD)]
            .spacing(gap::S)
            .align_y(Alignment::Center);
        if let Some(verifier) = crate::features::badge(p.verified(), self.badges) {
            line = line.push(ui::verified(verifier, theme::icon_size::L));
        }
        line
    }

    /// Their profile links as small buttons, each with the site's icon when it has
    /// one, that open in the browser.
    fn links(&self, links: Vec<ProfileLink>) -> Row<'a, M> {
        let mut out = Row::new().spacing(gap::S).align_y(Alignment::Center);
        for link in links {
            let icon: Element<'a, M> = match self.host.picture(link.icon.as_deref()) {
                Some(pic) => ui::picture(
                    pic,
                    theme::icon_size::M,
                    theme::icon_size::M,
                    theme::radius::S,
                ),
                None => ui::icon(Icon::Link, theme::icon_size::S, theme::Tone::Faint),
            };
            let label = row![icon, text(link.label()).size(size::SMALL)]
                .spacing(gap::XS + 2.0)
                .align_y(Alignment::Center);
            out = out.push(ui::tip(
                button(label)
                    .padding([gap::XS, gap::S])
                    .style(theme::ghost)
                    .on_press(self.msg(Msg::OpenUrl(link.url.clone()))),
                link.url,
            ));
        }
        out
    }

    /// Edit for your own profile; follow, message, mute, block and the rest for
    /// anyone else's.
    fn actions(&self, p: &'a Profile) -> Row<'a, M> {
        let mut actions = Row::new().spacing(gap::S).align_y(Alignment::Center);
        if p.did == self.me {
            actions = actions.push(
                ui::secondary("Edit profile")
                    .on_press_maybe(self.can_edit.then(|| self.msg(Msg::Edit))),
            );
        } else {
            let did = &p.did;
            let following = p.viewer.following.is_some();
            actions = actions
                .push(if following {
                    ui::secondary("Following").on_press(self.msg(Msg::Follow(did.clone(), false)))
                } else {
                    ui::primary_icon(Icon::Follow, "Follow")
                        .padding([gap::S - 1.0, gap::L])
                        .on_press(self.msg(Msg::Follow(did.clone(), true)))
                })
                .push(
                    ui::secondary(if self.chat_opening {
                        "Opening…"
                    } else {
                        "Message"
                    })
                    .on_press(self.msg(Msg::Message(did.clone()))),
                );
            if let Some(url) = p.germ() {
                actions = actions.push(ui::tip(
                    ui::secondary("Message on Germ")
                        .on_press(self.msg(Msg::OpenUrl(url.to_string()))),
                    "End-to-end encrypted messages, in the Germ app",
                ));
            }
            actions = actions
                .push(if p.viewer.muted {
                    ghost("Unmute", self.msg(Msg::Mute(did.clone(), false)))
                } else {
                    ghost("Mute", self.msg(Msg::Mute(did.clone(), true)))
                })
                .push(if p.viewer.blocking.is_some() {
                    ghost("Unblock", self.msg(Msg::Block(did.clone(), false)))
                } else {
                    ghost("Block", self.msg(Msg::Block(did.clone(), true)))
                })
                .push(ghost(
                    "Lists",
                    self.msg(Msg::AddToLists(did.clone(), p.name().to_string())),
                ))
                .push(ghost("Report", self.msg(Msg::Report(did.clone()))));
        }
        actions.push(ui::tip(
            icon_button(
                Icon::External,
                self.msg(Msg::OpenUrl(format!("https://bsky.app/profile/{}", p.did))),
            ),
            "Open on bsky.app",
        ))
    }
}
