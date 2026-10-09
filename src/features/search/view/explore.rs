//! What shows before a search runs: suggestions while typing, and the Explore page.

use super::{Host, View};
use crate::components::{self as ui, Icon, ghost, icon, icon_button, status};
use crate::features::search::message::Msg;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, container, row, space, text};
use iced::{Alignment, Element, Length, Padding};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// What shows while typing: run this search, earlier searches and tags that
    /// match, and people whose name or handle starts with it.
    pub(super) fn suggestions(&self) -> Element<'a, M> {
        let s = self.search;
        let q = s.query.trim();
        let ql = q.to_lowercase();
        let mut list = Column::new().spacing(gap::XXS);
        let mut shown: Vec<String> = Vec::new();
        list = list.push(suggestion(
            Icon::Search,
            format!("Search for \u{201c}{q}\u{201d}"),
            self.msg(Msg::For(q.to_string())),
        ));
        for r in self
            .recent
            .iter()
            .filter(|r| r.to_lowercase().contains(&ql) && r.as_str() != q)
            .take(3)
        {
            shown.push(r.to_lowercase());
            list = list.push(suggestion(
                Icon::Clock,
                r.clone(),
                self.msg(Msg::For(r.clone())),
            ));
        }
        if let Some(partial) = q.strip_prefix('#') {
            let partial = partial.to_lowercase();
            for t in self
                .tags
                .iter()
                .map(|t| format!("#{t}"))
                .filter(|t| {
                    let t = t.to_lowercase();
                    t[1..].starts_with(&partial) && t[1..] != partial && !shown.contains(&t)
                })
                .take(5)
            {
                list = list.push(suggestion(Icon::Hash, t.clone(), self.msg(Msg::For(t))));
            }
        }
        for a in &s.suggest {
            list = list.push(self.host.person(a));
        }
        container(list).padding([0.0, gap::M]).into()
    }

    /// The search screen before anything is searched.
    pub(super) fn explore(&self) -> Element<'a, M> {
        let s = self.search;
        let mut col = Column::new().spacing(gap::M).padding([gap::S, gap::XL]);

        if !self.recent.is_empty() {
            let mut list = Column::new().spacing(gap::XXS);
            for r in self.recent {
                list = list.push(
                    row![
                        button(
                            row![
                                icon(Icon::Clock, icon_size::M, Tone::Faint),
                                text(r.clone()).size(size::BODY),
                            ]
                            .spacing(gap::M)
                            .align_y(Alignment::Center),
                        )
                        .style(theme::row_button)
                        .padding([gap::S, gap::M])
                        .width(Length::Fill)
                        .on_press(self.msg(Msg::For(r.clone()))),
                        ui::tip(
                            icon_button(Icon::Close, self.msg(Msg::Forget(Some(r.clone())))),
                            "Remove",
                        ),
                    ]
                    .spacing(gap::XS)
                    .padding(Padding::ZERO.right(gap::XS))
                    .align_y(Alignment::Center),
                );
            }
            col = col
                .push(
                    row![
                        ui::eyebrow("Recent searches"),
                        space::horizontal(),
                        ghost("Clear all", self.msg(Msg::Forget(None))),
                    ]
                    .align_y(Alignment::Center),
                )
                .push(ui::card(list).padding(gap::XS))
                .push(space::vertical().height(gap::S));
        }

        col = col.push(
            row![
                icon(Icon::Trending, icon_size::M, Tone::Accent),
                ui::eyebrow("Trending"),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        );
        match &s.trends {
            None => col = col.push(status("Loading…")),
            Some(Err(e)) => col = col.push(self.host.retry(e.clone())),
            Some(Ok(trends)) if trends.is_empty() => {
                col = col.push(ui::muted("Nothing is trending right now."));
            }
            Some(Ok(trends)) => {
                let mut chips = Row::new().spacing(gap::XS);
                for (i, t) in trends.iter().enumerate() {
                    chips = chips.push(
                        button(
                            row![
                                ui::faint(format!("{}", i + 1)).size(size::SMALL),
                                text(t.name().to_string()).size(size::SMALL),
                            ]
                            .spacing(gap::XS + 2.0),
                        )
                        .padding([gap::XS + 2.0, gap::M + 2.0])
                        .style(theme::secondary)
                        .on_press(self.msg(Msg::OpenTrend(t.clone()))),
                    );
                }
                col = col.push(chips.wrap().vertical_spacing(gap::XS));
            }
        }

        if let Some(links) = self.host.top_links() {
            col = col
                .push(space::vertical().height(gap::S))
                .push(
                    row![
                        icon(Icon::Link, icon_size::M, Tone::Accent),
                        ui::eyebrow("Top links today"),
                        space::horizontal(),
                        ui::faint("From posts you saw"),
                    ]
                    .spacing(gap::S)
                    .align_y(Alignment::Center),
                )
                .push(ui::card(links).padding(gap::XS));
        }

        if !s.suggested_people.is_empty() {
            let mut people = Column::new().spacing(gap::XXS);
            for a in &s.suggested_people {
                people = people.push(self.host.person(a));
            }
            col = col
                .push(space::vertical().height(gap::S))
                .push(ui::eyebrow("People to follow"))
                .push(ui::card(people).padding(gap::XS));
        }

        if !self.feeds.is_empty() {
            let mut list = Column::new().spacing(gap::XXS);
            for g in self.feeds.iter().take(5) {
                list = list.push(self.host.feed(g));
            }
            col = col
                .push(space::vertical().height(gap::S))
                .push(
                    row![
                        ui::eyebrow("Feeds to try"),
                        space::horizontal(),
                        ghost("More feeds", self.msg(Msg::MoreFeeds)),
                    ]
                    .align_y(Alignment::Center),
                )
                .push(ui::card(list).padding(gap::XS));
        }
        col.into()
    }
}

/// A row in the suggestions: an icon and what picking it does.
fn suggestion<'a, M: Clone + 'a>(i: Icon, label: String, msg: M) -> Element<'a, M> {
    button(
        row![
            container(icon(i, icon_size::L, Tone::Muted))
                .width(40.0)
                .align_x(Alignment::Center),
            text(label).size(size::BODY),
        ]
        .spacing(gap::M)
        .align_y(Alignment::Center),
    )
    .style(theme::row_button)
    .padding([gap::S + 2.0, gap::M])
    .width(Length::Fill)
    .on_press(msg)
    .into()
}
