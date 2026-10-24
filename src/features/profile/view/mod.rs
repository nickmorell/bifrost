//! The profile screen: header, tabs, and what the tab shows. Posts, lists and starter
//! packs are drawn by the app, so these views build the app's own message type `M`
//! through [`View::lift`].

mod header;

use super::message::Msg;
use super::state::Profiles;
use super::tab::Tab;
use crate::components::{self as ui, pill, status};
use crate::features::Pictures;
use crate::model::{FeedGenerator, FeedSource, ListInfo, Profile};
use crate::theme::{self, gap, size};
use iced::widget::{Column, Row, button, column, container, row, text};
use iced::{Alignment, Element, Length};

/// What the profile screen needs from the app around it.
pub trait Host<M>: Pictures {
    /// The posts of `actor` that a timeline tab picks (see [`Tab::filter`]).
    fn posts<'a>(&'a self, actor: &str, tab: Tab) -> Element<'a, M>;
    /// The starter packs `actor` made, or `empty` when there are none.
    fn starter_packs<'a>(&'a self, actor: &str, empty: &'static str) -> Element<'a, M>;
    /// A list, as a card that opens it.
    fn list_card<'a>(&'a self, list: &'a ListInfo) -> Element<'a, M>;
    /// A bio, with its links and mentions working.
    fn linked_text<'a>(&'a self, text: &'a str) -> Element<'a, M>;
    /// Why something failed to load, with a way to try again.
    fn retry<'a>(&self, why: String) -> Element<'a, M>;
}

/// One profile with a tab open, unscrolled; the shell puts it in its scrollable.
pub struct View<'a, H, M> {
    pub profiles: &'a Profiles,
    /// The DID or handle the profile was opened with.
    pub actor: &'a str,
    pub tab: Tab,
    /// Your DID, to tell your own profile apart.
    pub me: &'a str,
    /// Whether your own profile is loaded, which editing needs.
    pub can_edit: bool,
    /// Whether a chat is being opened, which "Message" shows.
    pub chat_opening: bool,
    /// Whether to show verification checks.
    pub badges: bool,
    pub host: &'a H,
    /// Turns the screen's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    fn msg(&self, m: Msg) -> M {
        (self.lift)(m)
    }

    /// The whole screen.
    pub fn body(&self) -> Element<'a, M> {
        let profile = match self.profiles.by_actor.get(self.actor) {
            None => return status("Loading…"),
            Some(Err(e)) => return self.host.retry(e.clone()),
            Some(Ok(p)) => p,
        };
        let mut col = Column::new()
            .push(self.header(profile))
            .push(container(self.tabs(profile).wrap()).padding([gap::S, gap::L + gap::XS]));
        let blocked = profile.viewer.blocking.is_some() || profile.viewer.blocked_by;
        col = col.push(if blocked {
            status("Posts are hidden because of a block.")
        } else {
            self.tab_body()
        });
        col.into()
    }

    fn tabs(&self, profile: &Profile) -> Row<'a, M> {
        let pill_for = |label: &'static str, t: Tab| {
            pill(
                label,
                t == self.tab,
                self.msg(Msg::Tab(self.actor.to_string(), t)),
            )
        };
        let mut tabs = Row::new()
            .spacing(gap::XS)
            .push(pill_for("Posts", Tab::Posts))
            .push(pill_for("Replies", Tab::Replies))
            .push(pill_for("Media", Tab::Media));
        // Bluesky only lets you see your own likes.
        if profile.did == self.me {
            tabs = tabs.push(pill_for("Likes", Tab::Likes));
        }
        tabs.push(pill_for("Feeds", Tab::Feeds))
            .push(pill_for("Lists", Tab::Lists))
            .push(pill_for("Starter packs", Tab::StarterPacks))
    }

    /// What the open tab shows.
    fn tab_body(&self) -> Element<'a, M> {
        let actor = self.actor;
        match self.tab {
            Tab::Feeds => self.feed_cards(),
            Tab::Lists => match self.profiles.lists.get(actor) {
                None => status("Loading…"),
                Some(Err(e)) => self.host.retry(e.clone()),
                Some(Ok(lists)) if lists.is_empty() => status("No lists yet."),
                Some(Ok(lists)) => {
                    let mut c = Column::new().spacing(gap::XXS).padding([0.0, gap::M]);
                    for l in lists {
                        c = c.push(self.host.list_card(l));
                    }
                    c.into()
                }
            },
            Tab::StarterPacks => self.host.starter_packs(actor, "No starter packs yet."),
            Tab::Posts | Tab::Replies | Tab::Media | Tab::Likes => self.host.posts(actor, self.tab),
        }
    }

    /// The custom feeds the account made, each opening as a timeline.
    fn feed_cards(&self) -> Element<'a, M> {
        let feeds = match self.profiles.feeds.get(self.actor) {
            None => return status("Loading…"),
            Some(Err(e)) => return self.host.retry(e.clone()),
            Some(Ok(feeds)) if feeds.is_empty() => return status("No custom feeds yet."),
            Some(Ok(feeds)) => feeds,
        };
        let mut col = Column::new().spacing(gap::XXS).padding([0.0, gap::M]);
        for f in feeds {
            col = col.push(self.feed_card(f));
        }
        col.into()
    }

    fn feed_card(&self, f: &'a FeedGenerator) -> Element<'a, M> {
        let mut info = column![
            text(&f.display_name).size(size::BODY).font(theme::SEMIBOLD),
            ui::faint("Feed").size(size::CAPTION),
        ]
        .spacing(gap::XXS);
        if let Some(d) = f.description.as_deref().filter(|d| !d.is_empty()) {
            info = info.push(ui::muted(d.to_string()));
        }
        let open = Msg::OpenFeed(FeedSource::Generator(f.uri.clone()), f.display_name.clone());
        button(
            row![
                ui::feed_avatar(self.host.picture(f.avatar.as_deref()), 44.0),
                info.width(Length::Fill),
            ]
            .spacing(gap::M)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(gap::S + 2.0)
        .style(theme::row_button)
        .on_press(self.msg(open))
        .into()
    }
}
