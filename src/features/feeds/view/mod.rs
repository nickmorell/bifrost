//! The Feeds screen: pinned feeds in Home tab order, other saved feeds, and feeds to
//! discover.

mod tabs;

pub use tabs::Tabs;

use super::{Feeds, Msg};
use crate::components::{self as ui, Icon, ghost, icon_button, status};
use crate::features::Pictures;
use crate::model::{FeedGenerator, FeedSource, PrefEdit, SavedFeed};
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, column, row, space, text};
use iced::{Alignment, Element, Length, Padding};

/// The Feeds screen, unscrolled.
pub struct Screen<'a, P> {
    pub feeds: &'a Feeds,
    /// Your saved feeds, from your preferences.
    pub saved: &'a [SavedFeed],
    pub pictures: &'a P,
}

impl<'a, P: Pictures> Screen<'a, P> {
    pub fn view(self) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(gap::M).padding([gap::S, gap::XL]);

        let pinned: Vec<&SavedFeed> = self.saved.iter().filter(|f| f.pinned).collect();
        let mut pins = Column::new().spacing(gap::XXS);
        for (i, f) in pinned.iter().enumerate() {
            let mut actions = Row::new().spacing(gap::XXS).align_y(Alignment::Center);
            if i > 0 {
                actions = actions.push(icon_button(
                    Icon::ArrowUp,
                    Msg::Edit(PrefEdit::MoveFeed(f.source.clone(), -1)),
                ));
            }
            if i + 1 < pinned.len() {
                actions = actions.push(icon_button(
                    Icon::ArrowDown,
                    Msg::Edit(PrefEdit::MoveFeed(f.source.clone(), 1)),
                ));
            }
            if f.source != FeedSource::Following {
                actions = actions.push(ghost(
                    "Unpin",
                    Msg::Edit(PrefEdit::PinFeed(f.source.clone(), false)),
                ));
            }
            pins = pins.push(self.saved_row(&f.source, actions));
        }
        col = col
            .push(ui::eyebrow("Pinned"))
            .push(ui::muted(
                "Pinned feeds are the tabs on Home, in this order.",
            ))
            .push(ui::card(pins).padding(gap::XS));

        let others: Vec<&SavedFeed> = self.saved.iter().filter(|f| !f.pinned).collect();
        if !others.is_empty() {
            let mut list = Column::new().spacing(gap::XXS);
            for f in others {
                let actions = row![
                    ghost("Pin", Msg::Edit(PrefEdit::PinFeed(f.source.clone(), true))),
                    ghost("Remove", Msg::Edit(PrefEdit::RemoveFeed(f.source.clone()))),
                ]
                .spacing(gap::XXS)
                .align_y(Alignment::Center);
                list = list.push(self.saved_row(&f.source, actions));
            }
            col = col
                .push(space::vertical().height(gap::L))
                .push(ui::eyebrow("Saved"))
                .push(ui::card(list).padding(gap::XS));
        }

        col = col
            .push(space::vertical().height(gap::L))
            .push(ui::eyebrow("Discover"));
        match &self.feeds.suggested {
            None => col = col.push(status("Loading…")),
            Some(Err(e)) => col = col.push(ui::retry(e.clone(), Msg::SignInAgain, Msg::Retry)),
            Some(Ok(feeds)) => {
                let mut list = Column::new().spacing(gap::XXS);
                for g in feeds {
                    list = list.push(discover(self.pictures, self.saved, g));
                }
                if feeds.is_empty() {
                    list = list.push(status("No suggestions right now."));
                }
                col = col.push(ui::card(list).padding(gap::XS));
            }
        }
        col.into()
    }

    /// A saved feed's name and picture, from what was looked up or its tab.
    fn name_of(&self, source: &FeedSource) -> (String, Option<&'a str>) {
        let feeds = self.feeds;
        match source {
            FeedSource::Following => ("Following".into(), None),
            s => match feeds.info.get(s) {
                Some((n, a)) => (n.clone(), a.as_deref()),
                None => (
                    feeds
                        .tabs
                        .iter()
                        .find(|t| &t.source == s)
                        .map_or_else(|| "…".into(), |t| t.name.clone()),
                    None,
                ),
            },
        }
    }

    /// A saved feed with its controls on the right.
    fn saved_row(&self, source: &FeedSource, actions: Row<'a, Msg>) -> Element<'a, Msg> {
        let (name, avatar) = self.name_of(source);
        let kind = match source {
            FeedSource::Following => "Posts from people you follow",
            FeedSource::Generator(_) => "Feed",
            FeedSource::List(_) => "List",
        };
        let picture: Element<'a, Msg> = match source {
            FeedSource::Following => ui::mark(Icon::Home, 36.0, Tone::Accent),
            _ => ui::feed_avatar(self.pictures.picture(avatar), 36.0),
        };
        let open = match source {
            FeedSource::Following => Msg::Home,
            s => Msg::Open(s.clone(), name.clone()),
        };
        row![
            button(
                row![
                    picture,
                    column![
                        text(name).size(size::BODY).font(theme::MEDIUM),
                        ui::faint(kind).size(size::CAPTION),
                    ]
                    .spacing(gap::XXS)
                    .width(Length::Fill),
                ]
                .spacing(gap::M)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([gap::S, gap::S + 2.0])
            .style(theme::row_button)
            .on_press(open),
            actions,
        ]
        .spacing(gap::S)
        .padding(Padding::ZERO.right(gap::S))
        .align_y(Alignment::Center)
        .into()
    }
}

/// A feed you might add, with a Pin button or its state if already saved.
pub fn discover<'a>(
    pictures: &impl Pictures,
    saved: &[SavedFeed],
    g: &'a FeedGenerator,
) -> Element<'a, Msg> {
    let source = FeedSource::Generator(g.uri.clone());
    let action: Element<'a, Msg> = match saved.iter().find(|f| f.source == source) {
        Some(f) if f.pinned => ui::faint("Pinned").size(size::SMALL).into(),
        Some(_) => ghost("Pin", Msg::Edit(PrefEdit::PinFeed(source.clone(), true))).into(),
        None => ui::secondary("Pin")
            .on_press(Msg::Edit(PrefEdit::SaveFeed(source.clone(), true)))
            .into(),
    };
    let mut info = column![
        text(g.display_name.clone())
            .size(size::BODY)
            .font(theme::SEMIBOLD),
    ]
    .spacing(gap::XXS);
    if let Some(d) = g.description.as_deref().filter(|d| !d.is_empty()) {
        info = info.push(ui::muted(ui::truncate_lines(d, 3)));
    }
    row![
        button(
            row![
                ui::feed_avatar(pictures.picture(g.avatar.as_deref()), 44.0),
                info.width(Length::Fill),
            ]
            .spacing(gap::M)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(gap::S + 2.0)
        .style(theme::row_button)
        .on_press(Msg::Open(source, g.display_name.clone())),
        action,
    ]
    .spacing(gap::S)
    .padding(Padding::ZERO.right(gap::S))
    .align_y(Alignment::Center)
    .into()
}
