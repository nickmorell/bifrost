//! Posts kept on this computer (`storage::archive`): taking in what was loaded,
//! saving it now and then, and drawing kept posts for local search and offline
//! reading.

use super::util::{avatar, picture_of, relative_time};
use super::{App, Message, Route, nav};
use crate::components::{self as ui, Icon, divider, faint, icon, post_card as pc, status};
use crate::model::{Kept, Query, domain_of, top_links};
use crate::storage::archive::Archive;
use crate::theme::{self, Tone, gap, icon_size, size};
use iced::widget::{Column, button, column, container, row, text};
use iced::{Alignment, Element, Length, Padding};

/// Kept posts shown at once on the Offline reading screen.
const OFFLINE_SHOWN: usize = 300;
/// Links listed under "Top links today".
const TOP_LINKS: usize = 8;

impl App {
    /// Opens the posts kept for the account just signed in.
    pub(super) fn open_archive(&mut self) {
        self.archive = Archive::load(&self.my_did);
    }

    /// Keeps everything loaded now, and saves it if it changed.
    pub(super) fn keep_loaded(&mut self, now: bool) {
        if self.my_did.is_empty() {
            return;
        }
        let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let me = self.my_did.as_str();
        let kept: Vec<Kept> = self
            .timelines
            .posts()
            .chain(self.threads.posts())
            .chain(self.search.posts())
            .map(|p| Kept::of(p, me, &at))
            .collect();
        self.archive.keep(kept);
        self.archive.save(now);
    }

    /// Kept posts matching a search typed on the Search screen.
    pub(super) fn saved_search<'a>(&'a self, query: &str) -> Element<'a, Message> {
        let q = Query::parse(query);
        let found = self.archive.search(&q);
        let mut col = Column::new().push(
            column![
                faint(format!(
                    "{} found in the {} posts kept on this computer.",
                    found.len(),
                    self.archive.len()
                )),
                faint("Narrow it with from:handle, since:2026-01-31, until:, domain:site.com, is:liked, is:saved or is:mine."),
            ]
            .spacing(gap::XXS)
            .padding([gap::S, gap::XL]),
        );
        if found.is_empty() {
            return col.push(status("No kept posts match.")).into();
        }
        for (i, k) in found.into_iter().enumerate() {
            if i > 0 {
                col = col.push(divider());
            }
            col = col.push(self.kept_view(k));
        }
        col.into()
    }

    /// The links most shared in the last day of kept posts, each opening in the
    /// browser; `None` when no link was shared by two people.
    pub(super) fn top_links_view(&self) -> Option<Element<'_, Message>> {
        let since = chrono::Utc::now() - chrono::Duration::days(1);
        let links = top_links(self.archive.posts(), since, TOP_LINKS);
        if links.is_empty() {
            return None;
        }
        let mut col = Column::new().spacing(gap::XXS);
        for (i, l) in links.into_iter().enumerate() {
            let title = if l.title.is_empty() {
                ui::truncate(&l.url, 80)
            } else {
                l.title
            };
            col = col.push(
                button(
                    row![
                        faint(format!("{}", i + 1)).size(size::SMALL),
                        column![
                            text(title).size(size::BODY),
                            faint(format!("{} · shared by {} people", l.domain, l.people)),
                        ]
                        .spacing(gap::XXS)
                        .width(Length::Fill),
                    ]
                    .spacing(gap::M)
                    .align_y(Alignment::Center),
                )
                .style(theme::row_button)
                .padding([gap::S, gap::M])
                .width(Length::Fill)
                .on_press(Message::Nav(nav::Msg::OpenUrl(l.url))),
            );
        }
        Some(col.into())
    }

    /// "Can't reach Bluesky", with the way to the kept posts, while that is so.
    pub(super) fn offline_banner(&self) -> Option<Element<'_, Message>> {
        if !self.offline || matches!(self.route(), Route::Offline) {
            return None;
        }
        let mut r = row![
            icon(Icon::Computer, icon_size::L, Tone::Muted),
            ui::muted(if self.archive.is_empty() {
                "Can't reach Bluesky. Check your connection; BiFrost keeps trying.".to_string()
            } else {
                format!(
                    "Can't reach Bluesky. You can read the {} posts kept on this computer meanwhile.",
                    self.archive.len()
                )
            })
            .width(Length::Fill),
        ]
        .spacing(gap::M)
        .align_y(Alignment::Center);
        if !self.archive.is_empty() {
            r = r.push(
                ui::secondary("Read offline")
                    .on_press(Message::Nav(nav::Msg::OpenRoute(Route::Offline))),
            );
        }
        Some(ui::reading(
            container(ui::card(r).padding([gap::S, gap::M]))
                .padding(Padding::from([0.0, gap::L]).bottom(gap::S)),
        ))
    }

    /// The posts kept on this computer, newest first.
    pub(super) fn offline_view(&self) -> Element<'_, Message> {
        if self.archive.is_empty() {
            return self.scroll(status("No posts are kept on this computer yet."));
        }
        let mut posts: Vec<&Kept> = self.archive.posts().iter().collect();
        posts.sort_by(|a, b| b.created.cmp(&a.created));
        let mut list = Column::new().push(
            container(faint(format!(
                "The newest {} of the {} posts kept on this computer. Search them all from Search, under This computer.",
                posts.len().min(OFFLINE_SHOWN),
                posts.len()
            )))
            .padding([gap::S, gap::XL]),
        );
        for (i, k) in posts.into_iter().take(OFFLINE_SHOWN).enumerate() {
            if i > 0 {
                list = list.push(divider());
            }
            list = list.push(self.kept_view(k));
        }
        self.scroll(list)
    }

    /// A kept post: who, when, its text, pictures as descriptions and its link card.
    pub(super) fn kept_view<'a>(&'a self, k: &'a Kept) -> Element<'a, Message> {
        let mut head = row![
            text(if k.name.is_empty() {
                &k.handle
            } else {
                &k.name
            })
            .font(theme::SEMIBOLD)
            .size(size::BODY),
            faint(format!("@{}", k.handle)),
            faint("·"),
            faint(relative_time(&k.created)),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center);
        for mark in [(k.liked, "Liked"), (k.saved, "Saved")]
            .iter()
            .filter(|m| m.0)
            .map(|m| m.1)
        {
            head = head.push(ui::marker(mark));
        }
        let (font, body) = self.look().reading(size::BODY);
        let mut body_col = Column::new().spacing(gap::S).push(head.wrap());
        if !k.text.is_empty() {
            body_col = body_col.push(text(k.text.as_str()).font(font).size(body).line_height(1.5));
        }
        for (_, alt) in &k.images {
            let alt = if alt.is_empty() {
                "A picture"
            } else {
                alt.as_str()
            };
            body_col = body_col.push(faint(format!("[{alt}]")));
        }
        if let Some(c) = &k.card {
            body_col = body_col.push(ui::link_card(
                ui::Link {
                    domain: domain_of(&c.url).unwrap_or_default(),
                    title: &c.title,
                    description: ui::truncate(&c.description, 160),
                    thumb: c.thumb.as_deref().map(|t| picture_of(self, t)),
                },
                Message::Nav(nav::Msg::OpenUrl(c.url.clone())),
            ));
        }
        let open = Message::Nav(nav::Msg::OpenThread(k.uri.clone()));
        pc::card(pc::Card {
            avatar: avatar(self, k.avatar.as_deref(), pc::AVATAR),
            body: body_col,
            above: None,
            line_above: false,
            line_below: false,
            open: Some(open),
            menu: None,
        })
    }
}
