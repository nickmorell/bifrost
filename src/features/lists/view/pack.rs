//! Starter pack pages, the Lists screen, and list and pack cards.

use super::{confirm_delete, members, people, retry};
use crate::components::{self as ui, Icon, compact, ghost, icon, icon_button, pill, status};
use crate::features::Pictures;
use crate::features::lists::{Lists, Msg};
use crate::model::{AtUri, Did, FeedSource, ListInfo};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, column, container, row, space, text};
use iced::{Alignment, Element, Length};

impl Lists {
    /// A starter pack: who made it, Follow all, its feeds and its people.
    pub fn pack_view<'a>(
        &'a self,
        uri: &'a AtUri,
        host: &impl Pictures,
        me: &Did,
    ) -> Element<'a, Msg> {
        let p = match self.packs.get(uri) {
            None => return status("Loading…"),
            Some(Err(e)) => return retry(e.clone()),
            Some(Ok(p)) => p,
        };
        let mine = p.creator.did == *me;
        let list_uri = p.list.as_ref().map(|l| &l.uri);
        let screen = list_uri.and_then(|l| self.screens.get(l));

        let mut facts = vec![format!("Starter pack by @{}", p.creator.handle)];
        if let Some(n) = p.list.as_ref().and_then(|l| l.list_item_count) {
            facts.push(people(n));
        }
        if let Some(n) = p.joined_all_time_count.filter(|n| *n > 0) {
            facts.push(format!("{} joined", compact(n)));
        }
        let mut info = column![
            text(&p.record.name).size(size::TITLE).font(theme::SEMIBOLD),
            ui::faint(facts.join(" · ")),
        ]
        .spacing(gap::XXS);
        if let Some(d) = p
            .record
            .description
            .as_deref()
            .filter(|d| !d.trim().is_empty())
        {
            info = info.push(text(d).size(size::BODY));
        }
        let to_follow = screen.map_or(0, |s| {
            s.items
                .iter()
                .filter(|i| i.subject.viewer.following.is_none() && i.subject.did != *me)
                .count()
        });
        let following = self.following_all.contains(uri);
        let mut actions = Row::new().spacing(gap::S).align_y(Alignment::Center);
        actions = actions.push(
            ui::primary_icon(
                Icon::Follow,
                if following {
                    "Following…"
                } else if to_follow == 0 && screen.is_some() {
                    "Following everyone"
                } else {
                    "Follow all"
                },
            )
            .on_press_maybe((to_follow > 0 && !following).then(|| Msg::FollowAll(uri.clone()))),
        );
        if mine {
            actions = actions
                .push(ui::icon_action(
                    Icon::Pencil,
                    "Edit starter pack",
                    Msg::EditPack(uri.clone()),
                ))
                .push(ui::danger_icon_action(
                    Icon::Trash,
                    "Delete starter pack",
                    Msg::Delete(uri.clone()),
                ));
        }
        actions = actions.push(ui::tip(
            icon_button(
                Icon::External,
                Msg::OpenUrl(pack_web_url(&p.uri, &p.creator.handle)),
            ),
            "Open on bsky.app",
        ));
        if self.confirm_delete.as_deref() == Some(uri) {
            actions = actions.push(confirm_delete("Delete this starter pack?"));
        }

        let mut col = Column::new().spacing(gap::L).push(
            container(
                column![
                    row![
                        ui::avatar(host.picture(p.creator.avatar.as_deref()), 56.0),
                        info.width(Length::Fill)
                    ]
                    .spacing(gap::M)
                    .align_y(Alignment::Center),
                    actions,
                ]
                .spacing(gap::M),
            )
            .padding([gap::M, gap::XL]),
        );

        if !p.feeds.is_empty() {
            let mut feeds = Column::new().spacing(gap::XXS);
            for f in &p.feeds {
                feeds = feeds.push(
                    button(
                        row![
                            ui::feed_avatar(host.picture(f.avatar.as_deref()), 36.0),
                            text(&f.display_name).size(size::BODY).font(theme::MEDIUM),
                        ]
                        .spacing(gap::M)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding(gap::S)
                    .style(theme::row_button)
                    .on_press(Msg::OpenFeed(
                        FeedSource::Generator(f.uri.clone()),
                        f.display_name.clone(),
                    )),
                );
            }
            col = col.push(
                column![
                    container(ui::eyebrow("Feeds")).padding([0.0, gap::XL]),
                    feeds.padding([0.0, gap::M])
                ]
                .spacing(gap::S),
            );
        }

        col = col.push(container(ui::eyebrow("People")).padding([0.0, gap::XL]));
        col = col.push(match (list_uri, screen) {
            (Some(l), Some(s)) if s.list.is_some() => members(l, s, mine, host),
            (Some(_), _) => status("Loading…"),
            (None, _) => status("This pack has no people."),
        });
        col.into()
    }

    /// Your lists and starter packs, and buttons to make new ones. `my_lists` is what
    /// the app has of your lists so far.
    pub fn my_lists_view<'a>(
        &'a self,
        my_lists: Option<&'a Result<Vec<ListInfo>, String>>,
        host: &impl Pictures,
        me: &Did,
    ) -> Element<'a, Msg> {
        let packs = self.packs_tab;
        let mut col = Column::new().spacing(gap::L);
        let mut top = row![
            pill("Lists", !packs, Msg::PacksTab(false)),
            pill("Starter packs", packs, Msg::PacksTab(true)),
            space::horizontal(),
        ]
        .spacing(gap::XS)
        .align_y(Alignment::Center);
        top = if packs {
            top.push(ui::primary_icon(Icon::Plus, "New starter pack").on_press(Msg::NewPack))
        } else {
            top.push(ghost("New moderation list", Msg::NewList(true)))
                .push(ui::primary_icon(Icon::Plus, "New list").on_press(Msg::NewList(false)))
        };
        col = col.push(container(top).padding([gap::S, gap::XL]));

        if packs {
            col = col.push(self.pack_cards(me, "You haven't made a starter pack yet. A starter pack suggests people and feeds for newcomers to follow."));
        } else {
            col = col.push(match my_lists {
                None => status("Loading…"),
                Some(Err(e)) => retry(e.clone()),
                Some(Ok(lists)) => {
                    let mine: Vec<&ListInfo> = lists
                        .iter()
                        .filter(|l| !l.purpose.ends_with("referencelist"))
                        .collect();
                    if mine.is_empty() {
                        status("You haven't made a list yet. A list can be a feed of people's posts, or a moderation list to mute or block them together.")
                    } else {
                        let mut c = Column::new().spacing(gap::XXS).padding([0.0, gap::M]);
                        for l in mine {
                            c = c.push(list_card(host, l));
                        }
                        c.into()
                    }
                }
            });
        }
        col.into()
    }

    /// Someone's starter packs, as cards.
    pub fn pack_cards<'a>(&'a self, actor: &str, empty: &'static str) -> Element<'a, Msg> {
        match self.actor_packs.get(actor) {
            None => status("Loading…"),
            Some(Err(e)) => retry(e.clone()),
            Some(Ok(p)) if p.is_empty() => status(empty),
            Some(Ok(packs)) => {
                let mut c = Column::new().spacing(gap::XXS).padding([0.0, gap::M]);
                for p in packs {
                    let mut info = column![
                        text(&p.record.name).size(size::BODY).font(theme::SEMIBOLD),
                        ui::faint(match p.list_item_count {
                            Some(n) => format!("Starter pack · {} people", compact(n)),
                            None => "Starter pack".into(),
                        })
                        .size(size::CAPTION),
                    ]
                    .spacing(gap::XXS);
                    if let Some(d) = p.record.description.as_deref().filter(|d| !d.is_empty()) {
                        info = info.push(ui::muted(d.to_string()));
                    }
                    c = c.push(
                        button(
                            row![
                                container(icon(Icon::Users, icon_size::XXL, Tone::Accent))
                                    .center_x(44)
                                    .center_y(44)
                                    .style(theme::raised),
                                info.width(Length::Fill)
                            ]
                            .spacing(gap::M)
                            .align_y(Alignment::Center),
                        )
                        .width(Length::Fill)
                        .padding(gap::S + 2.0)
                        .style(theme::row_button)
                        .on_press(Msg::OpenPack(p.uri.clone())),
                    );
                }
                c.into()
            }
        }
    }
}

/// A list as a card that opens it.
pub fn list_card<'a>(host: &impl Pictures, l: &'a ListInfo) -> Element<'a, Msg> {
    let mut info = column![
        text(&l.name).size(size::BODY).font(theme::SEMIBOLD),
        ui::faint(if l.purpose.ends_with("modlist") {
            "Moderation list"
        } else {
            "List"
        })
        .size(size::CAPTION),
    ]
    .spacing(gap::XXS);
    if let Some(d) = l.description.as_deref().filter(|d| !d.is_empty()) {
        info = info.push(ui::muted(d.to_string()));
    }
    button(
        row![
            ui::feed_avatar(host.picture(l.avatar.as_deref()), 44.0),
            info.width(Length::Fill)
        ]
        .spacing(gap::M)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(gap::S + 2.0)
    .style(theme::row_button)
    .on_press(Msg::OpenList(l.uri.clone()))
    .into()
}

/// A starter pack's page on bsky.app.
pub(in crate::features::lists) fn pack_web_url(uri: &str, handle: &str) -> String {
    let rkey = uri.rsplit('/').next().unwrap_or_default();
    format!("https://bsky.app/starter-pack/{handle}/{rkey}")
}
