//! The ATstore screens: the directory, with search, tags and order, and a listing's
//! page (in `listing`).

mod listing;

use super::{Msg, Sort, Store};
use crate::components::{self as ui, Picture};
use crate::features::Pictures;
use crate::model::Listing;
use crate::theme::{self, gap, radius, size};
use iced::widget::{Column, Row, column, container, row, text};
use iced::{Alignment, Element, Length};

/// How big a listing's icon is in the list.
const ICON: f32 = 48.0;
/// Tags offered above the list, the most used first.
const TAGS: usize = 24;

/// The ATstore views, drawing [`Store`].
pub struct View<'a, P> {
    pub store: &'a Store,
    pub pictures: &'a P,
    /// With the data saver on, a listing's page leaves out its large pictures.
    pub data_saver: bool,
}

impl<'a, P: Pictures> View<'a, P> {
    /// The directory.
    pub fn browse(&self) -> Element<'a, Msg> {
        let s = self.store;
        let intro =
            ui::muted("Apps and services built on the AT Protocol, as listed on atstore.fyi.");
        if !s.has_catalog() {
            let body = match (&s.error, s.loading) {
                (_, true) => ui::status("Loading ATstore…"),
                (Some(why), false) => failed(why, Msg::Refresh),
                (None, false) => ui::status("Nothing is listed on ATstore yet."),
            };
            return column![intro, body]
                .spacing(gap::L)
                .padding([gap::S, gap::XL])
                .into();
        }

        let search = row![
            ui::field("Search ATstore", &s.query)
                .on_input(Msg::Query)
                .width(Length::Fill),
            ui::select(&Sort::ALL, Some(s.sort), Msg::Sort),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center);

        let found = s.matching();
        let mut list = Column::new().spacing(gap::XS);
        if found.is_empty() {
            list = list.push(ui::status("Nothing on ATstore matches that."));
        }
        for (i, l) in found.iter().take(s.shown).enumerate() {
            if i > 0 {
                list = list.push(ui::divider());
            }
            list = list.push(self.entry(l));
        }
        if found.len() > s.shown {
            list = list.push(
                container(ui::secondary("Show more").on_press(Msg::More))
                    .center_x(Length::Fill)
                    .padding([gap::M, 0.0]),
            );
        }

        let count = match found.len() {
            1 => "1 app".to_string(),
            n => format!("{n} apps"),
        };
        let mut about = vec![count];
        if s.loading {
            about.push("refreshing…".into());
        } else if !s.catalog.fetched.is_empty() {
            about.push(format!("updated {}", ui::relative_time(&s.catalog.fetched)));
        }
        let mut footer = Column::new()
            .spacing(gap::XS)
            .push(ui::faint(about.join(" · ")).size(size::SMALL));
        if let (Some(why), false) = (&s.error, s.loading) {
            footer = footer.push(
                ui::faint(format!("Couldn't refresh: {}", ui::sentence(why))).size(size::SMALL),
            );
        }

        column![intro, search, self.tags(), footer, list]
            .spacing(gap::M)
            .padding([gap::S, gap::XL])
            .into()
    }

    /// "All" and the most used tags, as pills.
    fn tags(&self) -> Element<'a, Msg> {
        let s = self.store;
        let mut pills =
            Row::new()
                .spacing(gap::XS)
                .push(ui::pill("All", s.tag.is_none(), Msg::Tag(None)));
        for (tag, _) in s.catalog.tags().into_iter().take(TAGS) {
            let on = s.tag.as_ref() == Some(&tag);
            pills = pills.push(ui::pill(label(&tag), on, Msg::Tag(Some(tag))));
        }
        ui::tab_strip("atstore-tags", pills)
    }

    /// One listing in the list: its icon, name, tagline and rating.
    fn entry(&self, l: &'a Listing) -> Element<'a, Msg> {
        let mut info = Column::new()
            .spacing(gap::XXS)
            .width(Length::Fill)
            .push(text(l.name.as_str()).size(size::BODY).font(theme::MEDIUM));
        if !l.tagline.is_empty() {
            info = info.push(ui::muted(ui::truncate_lines(&l.tagline, 2)).size(size::SMALL));
        }
        let facts = facts(l);
        if !facts.is_empty() {
            info = info.push(ui::faint(facts).size(size::SMALL));
        }
        ui::hand(
            row![self.icon(l, ICON), info]
                .spacing(gap::M)
                .padding([gap::S, 0.0])
                .align_y(Alignment::Center),
        )
        .on_press(Msg::Select(l.uri.clone()))
        .into()
    }

    /// A listing's icon, `side` square, or a placeholder until it loads.
    fn icon(&self, l: &Listing, side: f32) -> Element<'a, Msg> {
        let pic = self
            .pictures
            .picture(l.icon_url.as_deref())
            .unwrap_or(Picture::Waiting { pulse: false });
        ui::picture(pic, side, side, radius::M)
    }
}

/// Its category, when that isn't just its own name, and its rating:
/// "Bluesky Feed Generators · Rated 4.8 · 9 reviews".
fn facts(l: &Listing) -> String {
    let mut out = Vec::new();
    if !l.category.is_empty() && !l.category.eq_ignore_ascii_case(&l.name) {
        out.push(l.category.clone());
    }
    if let Some(stars) = l.stars() {
        out.push(format!("Rated {stars:.1}"));
        out.push(match l.review_count {
            1 => "1 review".to_string(),
            n => format!("{} reviews", ui::compact(n)),
        });
    }
    out.join(" · ")
}

/// A tag as words: "bluesky client" -> "Bluesky client".
fn label(tag: &str) -> String {
    let words = tag.replace(['-', '_'], " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Something that couldn't be read, with "Try again".
fn failed<'a>(why: &str, again: Msg) -> Element<'a, Msg> {
    column![
        ui::muted(format!("Couldn't read ATstore. {}", ui::sentence(why))),
        ui::secondary("Try again").on_press(again),
    ]
    .spacing(gap::M)
    .into()
}
