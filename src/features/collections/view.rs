//! The Collections screen and a collection's page.

use super::{Msg, Screen};
use crate::components::{self as ui, Icon, divider, faint, ghost, icon, status};
use crate::model::{Collection, PostView};
use crate::theme::{self, Tone, gap, icon_size, size};
use iced::widget::{Column, column, container, row, text};
use iced::{Alignment, Element, Length};

/// What a collection's page needs from the app around it.
pub trait Host<M> {
    /// A post with its actions.
    fn post<'a>(&'a self, post: &'a PostView) -> Element<'a, M>;
}

/// The Collections screen: making one, and the list.
pub struct List<'a> {
    pub screen: &'a Screen,
    pub collections: &'a [Collection],
}

impl<'a> List<'a> {
    pub fn view(self) -> Element<'a, Msg> {
        let s = self.screen;
        let create = (!s.name.trim().is_empty()).then_some(Msg::Create);
        let mut new = column![
            row![
                ui::field("New collection", &s.name)
                    .on_input(Msg::Name)
                    .on_submit_maybe(create.clone()),
                ui::primary("Create").on_press_maybe(create),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center)
        ]
        .spacing(gap::XS);
        new = new.push(faint(if s.pending.is_some() {
            "The post you picked goes into it."
        } else {
            "Collections are kept only on this computer. Add posts from a post's … menu."
        }));
        let mut col = Column::new()
            .spacing(gap::S)
            .padding([0.0, gap::L])
            .push(ui::card(new).padding(gap::L));
        if self.collections.is_empty() {
            return col.push(status("No collections yet.")).into();
        }
        for c in self.collections {
            let count = match c.posts.len() {
                1 => "1 post".to_string(),
                n => format!("{n} posts"),
            };
            let line = row![
                icon(Icon::Folder, icon_size::L, Tone::Accent),
                column![
                    text(c.name.as_str()).font(theme::SEMIBOLD).size(size::BODY),
                    faint(count)
                ]
                .spacing(gap::XXS)
                .width(Length::Fill),
                ghost("Delete", Msg::Delete(c.id)),
                ui::secondary("Open").on_press(Msg::Open(c.id)),
            ]
            .spacing(gap::M)
            .align_y(Alignment::Center);
            col = col
                .push(ui::hand(ui::card(line).padding([gap::M, gap::L])).on_press(Msg::Open(c.id)));
        }
        col.into()
    }
}

/// One collection's posts, each with a way to take it out.
pub struct Page<'a, H, M> {
    pub screen: &'a Screen,
    pub collection: &'a Collection,
    pub host: &'a H,
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<M>, M: Clone + 'a> Page<'a, H, M> {
    pub fn view(self) -> Element<'a, M> {
        let c = self.collection;
        let posts = match self.screen.posts(c.id) {
            None => return status("Loading…"),
            Some(Err(why)) => return status(format!("Couldn't load the posts. {why}")),
            Some(Ok(posts)) => posts,
        };
        if posts.is_empty() {
            return status("Nothing in this collection yet. Add posts from a post's … menu.");
        }
        let mut col = Column::new();
        for (i, p) in posts.iter().enumerate() {
            if i > 0 {
                col = col.push(divider());
            }
            col = col.push(self.host.post(p)).push(
                container(ghost(
                    "Remove from collection",
                    (self.lift)(Msg::Toggle(c.id, p.uri.clone())),
                ))
                .padding([0.0, gap::XL]),
            );
        }
        let gone = c.posts.len().saturating_sub(posts.len());
        if gone > 0 {
            col = col.push(status(format!(
                "{gone} more {} deleted or can't be seen.",
                if gone == 1 { "was" } else { "were" }
            )));
        }
        col.into()
    }
}
