//! The search screen: the search box, then suggestions while typing, results once
//! searched, or the Explore page. Posts, people and feeds are drawn by the app, so
//! these views build the app's own message type `M` through [`View::lift`].

mod explore;

use super::message::Msg;
use super::state::{INPUT, Search};
use super::tab::Tab;
use crate::components::{self as ui, Icon, divider, ghost, icon, icon_button, pill, status};
use crate::model::{Author, FeedGenerator, PostView};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, column, container, row, space, text};
use iced::{Alignment, Element, Length, Padding};

/// What the search screen needs from the app around it.
pub trait Host<M> {
    /// A found post with its actions.
    fn post<'a>(&'a self, post: &'a PostView) -> Element<'a, M>;
    /// An account as a row that opens its profile.
    fn person<'a>(&'a self, author: &'a Author) -> Element<'a, M>;
    /// A feed as a row that opens it, with a way to pin it.
    fn feed<'a>(&'a self, feed: &'a FeedGenerator) -> Element<'a, M>;
    /// Why something failed to load, with a way to try again.
    fn retry<'a>(&self, why: String) -> Element<'a, M>;
    /// Posts kept on this computer that match `query`.
    fn saved<'a>(&'a self, query: &'a str) -> Element<'a, M>;
    /// The links most shared in posts seen today, if any were.
    fn top_links(&self) -> Option<Element<'_, M>>;
    /// The scrolling part of the screen.
    fn scroll<'a>(&self, content: Element<'a, M>) -> Element<'a, M>;
}

/// The search screen.
pub struct View<'a, H, M> {
    pub search: &'a Search,
    /// Recent searches, newest first.
    pub recent: &'a [String],
    /// Tags recently used, to suggest while typing a `#tag`.
    pub tags: &'a [String],
    /// The muted word that mutes this #tag page's tag, if one does.
    pub muted_tag: Option<&'a str>,
    /// Feeds to suggest on the Explore page.
    pub feeds: &'a [FeedGenerator],
    pub host: &'a H,
    /// Turns the screen's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    fn msg(&self, m: Msg) -> M {
        (self.lift)(m)
    }

    /// The whole screen.
    pub fn screen(&self) -> Element<'a, M> {
        let s = self.search;
        let tab = s.tab.unwrap_or(Tab::Top);
        let lift = self.lift;
        let mut input = row![
            container(icon(Icon::Search, icon_size::M, Tone::Faint)).padding([0.0, gap::XS]),
            ui::field("Search posts, people, feeds, #tags", &s.query)
                .id(INPUT)
                .on_input(move |q| lift(Msg::Input(q)))
                .on_submit(self.msg(Msg::Submit)),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center)
        .padding([0.0, gap::XL]);
        if !s.query.is_empty() || s.searched() {
            input = input.push(ui::tip(
                icon_button(Icon::Close, self.msg(Msg::Clear)),
                "Clear",
            ));
        }
        input = input.push(ui::primary("Search").on_press(self.msg(Msg::Submit)));

        let mut header = column![input].spacing(gap::M);
        let body: Element<'a, M> = if s.typing && !s.query.trim().is_empty() {
            self.suggestions()
        } else if s.searched() {
            let pill_for =
                |label: &'static str, t: Tab| pill(label, tab == t, self.msg(Msg::Tab(t)));
            let mut tabs =
                row![pill_for("Top", Tab::Top), pill_for("Latest", Tab::Latest)].spacing(gap::XS);
            if s.tag().is_none() {
                tabs = tabs
                    .push(pill_for("People", Tab::People))
                    .push(pill_for("Feeds", Tab::Feeds));
            }
            tabs = tabs.push(pill_for("This computer", Tab::Saved));
            header = header.push(container(tabs).padding([0.0, gap::L + gap::XS]));
            self.results(tab)
        } else {
            self.explore()
        };
        column![
            ui::reading(header),
            space::vertical().height(gap::S),
            self.host.scroll(body),
        ]
        .into()
    }

    fn results(&self, tab: Tab) -> Element<'a, M> {
        let s = self.search;
        let mut results = Column::new();
        if let Some(tag) = s.tag() {
            results = results.push(self.tag_header(tag));
        }
        match tab {
            Tab::Top | Tab::Latest => {
                if s.posts_sort == Some(tab) {
                    for (i, p) in s.posts.iter().enumerate() {
                        if i > 0 {
                            results = results.push(divider());
                        }
                        results = results.push(self.host.post(p));
                    }
                    if !s.posts_loading && s.posts.is_empty() {
                        results = results.push(status("No posts found."));
                    }
                }
            }
            Tab::People => {
                let mut people = Column::new().spacing(gap::XXS);
                for a in &s.people {
                    people = people.push(self.host.person(a));
                }
                results = results.push(container(people).padding([0.0, gap::M]));
                if !s.people_loading && s.people.is_empty() {
                    results = results.push(status("No people found."));
                }
            }
            Tab::Saved => results = results.push(self.host.saved(&s.submitted)),
            Tab::Feeds => {
                let mut feeds = Column::new().spacing(gap::XXS);
                for g in &s.feeds {
                    feeds = feeds.push(self.host.feed(g));
                }
                results = results.push(container(feeds).padding([0.0, gap::M]));
                if !s.feeds_loading && s.feeds.is_empty() {
                    results = results.push(status("No feeds found."));
                }
            }
        }
        if s.loading(tab) {
            results = results.push(status("Searching…"));
        }
        results.into()
    }

    /// The top of a #tag search: what it is, and what you can do with it.
    fn tag_header(&self, tag: &'a str) -> Element<'a, M> {
        let mute = match self.muted_tag {
            Some(word) => ghost(
                "Unmute tag",
                self.msg(Msg::MuteTag(word.to_string(), false)),
            ),
            None => ghost("Mute tag", self.msg(Msg::MuteTag(tag.to_string(), true))),
        };
        container(
            ui::card(
                row![
                    ui::mark(Icon::Hash, 44.0, Tone::Accent),
                    column![
                        text(format!("#{tag}"))
                            .size(size::TITLE)
                            .font(theme::SEMIBOLD),
                        ui::muted(if self.muted_tag.is_some() {
                            "Muted. Posts with this tag are hidden everywhere else."
                        } else {
                            "Posts tagged with this hashtag."
                        }),
                    ]
                    .spacing(gap::XXS)
                    .width(Length::Fill),
                    mute,
                    ui::secondary("Post with tag")
                        .on_press(self.msg(Msg::PostWithTag(tag.to_string()))),
                ]
                .spacing(gap::M)
                .align_y(Alignment::Center),
            )
            .padding(gap::L),
        )
        .padding(Padding::ZERO.left(gap::XL).right(gap::XL).bottom(gap::M))
        .into()
    }
}
