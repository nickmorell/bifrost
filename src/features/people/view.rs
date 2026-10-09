//! A list of accounts, and one account's row.

use super::state::List;
use super::{Msg, People};
use crate::components::{self as ui, ghost, status};
use crate::features::Pictures;
use crate::model::{Author, PeopleKind};
use crate::theme::{self, gap, size};
use iced::widget::{Column, button, column, row, text};
use iced::{Alignment, Element, Length};

/// The list `kind` of `actor`, unscrolled.
pub struct View<'a, P> {
    pub people: &'a People,
    pub actor: &'a str,
    pub kind: PeopleKind,
    pub pictures: &'a P,
}

impl<'a, P: Pictures> View<'a, P> {
    pub fn view(self) -> Element<'a, Msg> {
        let col = Column::new().spacing(gap::XXS).padding([0.0, gap::M]);
        match self.people.lists.get(&(self.actor.to_string(), self.kind)) {
            None => col.push(status("Loading…")).into(),
            Some(list) => self.list(col, list),
        }
    }

    fn list(&self, mut col: Column<'a, Msg>, list: &'a List) -> Element<'a, Msg> {
        let kind = self.kind;
        for a in &list.items {
            // Muted and blocked lists keep people you just unmuted or unblocked, so a
            // slip can be undone from the same row.
            let action = match kind {
                PeopleKind::Muted => Some(if a.viewer.muted {
                    ghost("Unmute", Msg::Mute(a.did.clone(), false))
                } else {
                    ghost("Mute", Msg::Mute(a.did.clone(), true))
                }),
                PeopleKind::Blocked => Some(if a.viewer.blocking.is_some() {
                    ghost("Unblock", Msg::Block(a.did.clone(), false))
                } else {
                    ghost("Block", Msg::Block(a.did.clone(), true))
                }),
                _ => None,
            };
            col = col.push(match action {
                Some(b) => row![person(self.pictures, a), b]
                    .spacing(gap::S)
                    .align_y(Alignment::Center)
                    .into(),
                None => person(self.pictures, a),
            });
        }
        if let Some(e) = &list.error {
            let (actor, kind) = (self.actor.to_string(), self.kind);
            col = col.push(ui::retry(
                e.clone(),
                Msg::SignInAgain,
                Msg::Retry(actor, kind),
            ));
        } else if list.loading {
            col = col.push(status("Loading…"));
        } else if list.items.is_empty() {
            col = col.push(status(match kind {
                PeopleKind::Followers => "No followers yet.",
                PeopleKind::Following => "Not following anyone yet.",
                PeopleKind::Muted => "You haven't muted anyone.",
                PeopleKind::Blocked => "You haven't blocked anyone.",
            }));
        }
        col.into()
    }
}

/// One account: picture, name and handle. Clicking it opens the profile.
pub fn person<'a>(pictures: &impl Pictures, a: &'a Author) -> Element<'a, Msg> {
    button(
        row![
            ui::avatar(pictures.picture(a.avatar.as_deref()), 40.0),
            column![
                text(a.name()).size(size::BODY).font(theme::SEMIBOLD),
                ui::faint(format!("@{}", a.handle)),
            ]
            .spacing(gap::XXS),
        ]
        .spacing(gap::M)
        .align_y(Alignment::Center),
    )
    .style(theme::row_button)
    .padding([gap::S + 2.0, gap::M])
    .width(Length::Fill)
    .on_press(Msg::Profile(a.did.clone()))
    .into()
}
