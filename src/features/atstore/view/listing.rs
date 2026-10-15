//! A listing's page: its pictures, what it is, where it lives and its other links.

use super::{View, facts, label};
use crate::components::{self as ui, Picture};
use crate::features::Pictures;
use crate::features::atstore::{Msg, Reading};
use crate::model::{AtUri, Listing, ListingDetail};
use crate::theme::{self, gap, radius, size};
use iced::widget::{Column, Row, image, row, scrollable, text};
use iced::{Alignment, ContentFit, Element, Length, Padding};

/// How big the icon is at the top of the page.
const ICON: f32 = 72.0;
/// How tall the wide picture is.
const HERO_H: f32 = 240.0;
/// How big each screenshot is.
const SHOT_W: f32 = 300.0;
const SHOT_H: f32 = 190.0;

impl<'a, P: Pictures> View<'a, P> {
    /// The page of the listing at `uri`.
    pub fn listing(&self, uri: &AtUri) -> Element<'a, Msg> {
        let s = self.store;
        let reading = s.pages.get(uri);
        let Some(l) = s.listing(uri) else {
            return match reading {
                Some(Reading::Failed(why)) => super::failed(why, Msg::Reload(uri.clone())),
                _ => ui::status("Loading…"),
            };
        };
        let detail = s.catalog.details.get(uri);

        let mut col = Column::new().spacing(gap::L).padding([gap::S, gap::XL]);
        if let Some(hero) = self.large(l.hero_image_url.as_deref()) {
            col = col.push(
                image(hero)
                    .width(Length::Fill)
                    .height(HERO_H)
                    .content_fit(ContentFit::Cover)
                    .border_radius(radius::M),
            );
        }
        col = col.push(self.header(l));
        col = col.push(actions(l, detail));
        match reading {
            Some(Reading::Loading) => col = col.push(ui::faint("Loading its page…")),
            Some(Reading::Failed(why)) => {
                col = col.push(
                    row![
                        ui::faint(format!("Couldn't load its page: {}", ui::sentence(why)))
                            .width(Length::Fill),
                        ui::ghost("Try again", Msg::Reload(uri.clone())),
                    ]
                    .align_y(Alignment::Center),
                );
            }
            None => {}
        }
        for para in l.description.split("\n\n").filter(|p| !p.trim().is_empty()) {
            col = col.push(text(para.trim()).size(size::BODY).line_height(1.6));
        }
        if let Some(d) = detail {
            if let Some(shots) = self.screenshots(d) {
                col = col.push(shots);
            }
            col = col.push(links(d));
        }
        col.push(ui::faint("Listed on atstore.fyi").size(size::SMALL))
            .into()
    }

    fn header(&self, l: &'a Listing) -> Element<'a, Msg> {
        let mut about = Column::new().spacing(gap::XXS).width(Length::Fill).push(
            text(l.name.as_str())
                .size(size::TITLE)
                .font(theme::SEMIBOLD),
        );
        if !l.tagline.is_empty() {
            about = about.push(ui::muted(l.tagline.as_str()));
        }
        let facts = facts(l);
        if !facts.is_empty() {
            about = about.push(ui::faint(facts).size(size::SMALL));
        }
        if !l.app_tags.is_empty() {
            let tags: Vec<String> = l.app_tags.iter().map(|t| label(t)).collect();
            about = about.push(ui::faint(tags.join(" · ")).size(size::SMALL));
        }
        row![self.icon(l, ICON), about]
            .spacing(gap::L)
            .align_y(Alignment::Center)
            .into()
    }

    /// Its screenshots side by side, when it has some and they may be shown.
    fn screenshots(&self, d: &'a ListingDetail) -> Option<Element<'a, Msg>> {
        if self.data_saver || d.screenshots.is_empty() {
            return None;
        }
        let mut shots = Row::new().spacing(gap::S);
        for url in &d.screenshots {
            let pic = self
                .pictures
                .picture(Some(url))
                .unwrap_or(Picture::Waiting { pulse: false });
            shots = shots.push(ui::picture(pic, SHOT_W, SHOT_H, radius::M));
        }
        Some(
            scrollable(shots.padding(Padding::ZERO.bottom(gap::S)))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new(),
                ))
                .width(Length::Fill)
                .into(),
        )
    }

    /// A large picture, once loaded, unless the data saver is on.
    fn large(&self, url: Option<&str>) -> Option<image::Handle> {
        if self.data_saver {
            return None;
        }
        match self.pictures.picture(url)? {
            Picture::Loaded(h) => Some(h),
            _ => None,
        }
    }
}

/// "Open" the app, and its Bluesky account.
fn actions<'a>(l: &'a Listing, detail: Option<&'a ListingDetail>) -> Element<'a, Msg> {
    let mut buttons = Row::new().spacing(gap::S);
    let site = detail.and_then(|d| d.external_url.as_deref());
    if let Some(url) = site.filter(|u| is_web(u)) {
        let what = match host_of(url) {
            Some(host) => format!("Open {host}"),
            None => "Open".to_string(),
        };
        buttons = buttons.push(ui::primary(what).on_press(Msg::OpenUrl(url.to_string())));
    }
    if let Some(handle) = l.product_account_handle.as_deref() {
        buttons = buttons.push(
            ui::secondary(format!("@{handle}")).on_press(Msg::OpenProfile(handle.to_string())),
        );
    }
    buttons.into()
}

/// Its other pages, each opening in the browser.
fn links(d: &ListingDetail) -> Element<'_, Msg> {
    let mut col = Column::new().spacing(gap::XXS);
    let links: Vec<&str> = d
        .links
        .iter()
        .map(|l| l.uri.as_str())
        .filter(|u| is_web(u))
        .collect();
    if links.is_empty() {
        return col.into();
    }
    col = col.push(text("Links").size(size::LARGE).font(theme::SEMIBOLD));
    for url in links {
        let shown = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/');
        col = col.push(ui::ghost(shown, Msg::OpenUrl(url.to_string())));
    }
    col.into()
}

/// The site an address is on, without `www.`.
fn host_of(url: &str) -> Option<String> {
    let host = url::Url::parse(url).ok()?.host_str()?.to_ascii_lowercase();
    Some(host.trim_start_matches("www.").to_string())
}

/// Whether an address is a web page, the only kind these pages open.
fn is_web(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}
