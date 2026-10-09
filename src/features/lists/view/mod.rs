//! The lists feature's views: a list's page, people, and the shared bits.

mod dialogs;
pub use dialogs::{add_to, editor};
pub(super) mod pack;
pub use pack::list_card;

use super::message::Msg;
use super::state::{ListScreen, ListTab, Lists};
use crate::components::{self as ui, Icon, compact, ghost, pill, status};
use crate::features::Pictures;
use crate::model::{AtUri, Author, Did, ListView};
use crate::theme::{self, gap, size};
use iced::widget::{Column, Row, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

/// A list's page, or what to show instead.
pub enum ListParts<'a> {
    /// Still loading, or failed to load.
    Status(Element<'a, Msg>),
    /// The header and tabs, then the list's people; `None` on the Posts tab, where
    /// the app shows the list's timeline instead.
    Page {
        top: Vec<Element<'a, Msg>>,
        people: Option<Element<'a, Msg>>,
    },
}

impl Lists {
    /// One list: its header, then its posts or its people.
    pub fn list_view<'a>(
        &'a self,
        uri: &'a AtUri,
        host: &impl Pictures,
        me: &Did,
    ) -> ListParts<'a> {
        let Some(s) = self.screens.get(uri) else {
            return ListParts::Status(status("Loading…"));
        };
        let l = match &s.list {
            None => return ListParts::Status(status("Loading…")),
            Some(Err(e)) => return ListParts::Status(retry(e.clone())),
            Some(Ok(l)) => l,
        };
        let mine = l.creator.did == *me;
        let mut top = vec![self.list_header(l, mine, host)];
        if !l.is_mod() {
            let tab = |label, t| pill(label, s.tab == t, Msg::Tab(uri.clone(), t));
            top.push(
                row![tab("Posts", ListTab::Posts), tab("People", ListTab::People)]
                    .spacing(gap::XS)
                    .padding([0.0, gap::XL])
                    .into(),
            );
        }
        let people = (s.tab != ListTab::Posts || l.is_mod()).then(|| members(uri, s, mine, host));
        ListParts::Page { top, people }
    }

    fn list_header<'a>(
        &self,
        l: &'a ListView,
        mine: bool,
        host: &impl Pictures,
    ) -> Element<'a, Msg> {
        let count = l
            .list_item_count
            .map(|n| format!(" · {}", people(n)))
            .unwrap_or_default();
        let mut info = column![
            text(&l.name).size(size::TITLE).font(theme::SEMIBOLD),
            ui::faint(format!(
                "{} by @{}{count}",
                l.kind_label(),
                l.creator.handle
            )),
        ]
        .spacing(gap::XXS);
        if let Some(d) = l.description.as_deref().filter(|d| !d.trim().is_empty()) {
            info = info.push(text(d).size(size::BODY));
        }
        let mut actions = Row::new().spacing(gap::S).align_y(Alignment::Center);
        if l.is_mod() {
            actions = actions
                .push(if l.viewer.muted {
                    ui::secondary("Unmute list").on_press(Msg::Mute(l.uri.clone(), false))
                } else {
                    ui::secondary("Mute list").on_press(Msg::Mute(l.uri.clone(), true))
                })
                .push(match &l.viewer.blocked {
                    Some(r) => ghost("Unblock list", Msg::Block(l.uri.clone(), Some(r.clone()))),
                    None => ghost("Block list", Msg::Block(l.uri.clone(), None)),
                });
        }
        if mine {
            actions = actions
                .push(ui::icon_action(
                    Icon::Pencil,
                    "Edit list",
                    Msg::Edit(l.uri.clone()),
                ))
                .push(ui::danger_icon_action(
                    Icon::Trash,
                    "Delete list",
                    Msg::Delete(l.uri.clone()),
                ));
        }
        if self.confirm_delete.as_deref() == Some(l.uri.as_str()) {
            actions = actions.push(confirm_delete("Delete this list?"));
        }
        container(
            column![
                row![
                    ui::feed_avatar(host.picture(l.avatar.as_deref()), 56.0),
                    info.width(Length::Fill)
                ]
                .spacing(gap::M)
                .align_y(Alignment::Center),
                actions,
            ]
            .spacing(gap::M),
        )
        .padding([gap::M, gap::XL])
        .into()
    }
}

/// The people on a list, with search-and-add and Remove on your own lists.
fn members<'a>(
    uri: &'a AtUri,
    s: &'a ListScreen,
    mine: bool,
    host: &impl Pictures,
) -> Element<'a, Msg> {
    let mut col = Column::new().spacing(gap::XXS).padding([0.0, gap::M]);
    if mine {
        col = col.push(
            container(
                text_input("Add people: search by name or handle", &s.query)
                    .on_input(move |q| Msg::Query(uri.clone(), q))
                    .padding(gap::S + 2.0)
                    .size(size::BODY)
                    .style(theme::input),
            )
            .padding([gap::S, gap::S]),
        );
        for a in &s.results {
            let on = s.items.iter().any(|i| i.subject.did == a.did);
            let busy = s.busy.contains(&a.did);
            let action: Element<'a, Msg> = if on {
                ui::faint("On the list").into()
            } else {
                ui::secondary(if busy { "Adding…" } else { "Add" })
                    .on_press_maybe((!busy).then(|| Msg::Add(uri.clone(), a.clone())))
                    .into()
            };
            col = col.push(person(host, a, Some(action)));
        }
        if !s.results.is_empty() {
            col = col.push(ui::divider());
        }
    }
    if s.items.is_empty() {
        col = col.push(status(if mine {
            "No one on this list yet. Search above to add people."
        } else {
            "No one on this list yet."
        }));
    }
    for i in &s.items {
        let action: Option<Element<'a, Msg>> = mine.then(|| {
            let busy = s.busy.contains(&i.subject.did);
            ghost(
                if busy { "Removing…" } else { "Remove" },
                Msg::Remove(uri.clone(), i.clone()),
            )
            .into()
        });
        col = col.push(person(host, &i.subject, action));
    }
    if s.cursor.is_some() {
        col = col.push(
            container(
                ui::secondary(if s.loading { "Loading…" } else { "Show more" })
                    .on_press(Msg::More(uri.clone())),
            )
            .center_x(Length::Fill)
            .padding(gap::M),
        );
    }
    col.into()
}

/// Something that failed to load, with "Sign in again" or "Try again".
fn retry<'a>(why: String) -> Element<'a, Msg> {
    ui::retry(why, Msg::SignInAgain, Msg::Retry)
}

/// "1 person", "12K people".
fn people(n: u64) -> String {
    format!(
        "{} {}",
        compact(n),
        if n == 1 { "person" } else { "people" }
    )
}

fn confirm_delete(question: &str) -> Element<'_, Msg> {
    row![
        ui::muted(question),
        ui::danger("Delete", Msg::ConfirmDelete),
        ghost("Keep", Msg::CancelDelete),
    ]
    .spacing(gap::S)
    .align_y(Alignment::Center)
    .into()
}

/// A person on a list, opening their profile, with an optional button at the end.
fn person<'a>(
    host: &impl Pictures,
    a: &'a Author,
    action: Option<Element<'a, Msg>>,
) -> Element<'a, Msg> {
    let body = button(
        row![
            ui::avatar(host.picture(a.avatar.as_deref()), 40.0),
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
    .on_press(Msg::OpenProfile(a.did.clone()));
    match action {
        Some(b) => row![body, b]
            .spacing(gap::S)
            .align_y(Alignment::Center)
            .into(),
        None => body.into(),
    }
}
