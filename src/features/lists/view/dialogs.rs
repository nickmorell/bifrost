//! The dialogs: making or editing a list or pack, and "Add to lists".

use super::retry;
use crate::components::{self as ui, Icon, ghost, icon_button, status};
use crate::features::lists::{AddTo, Editing, Editor, Msg};
use crate::model::{AtUri, MAX_PACK_FEEDS};
use crate::theme::{self, gap, size};
use iced::widget::{Column, checkbox, column, container, row, scrollable, space, text};
use iced::{Alignment, Element, Length};

/// Making or editing a list or starter pack. `saved` is your saved feeds with their
/// names, which a starter pack can include.
pub fn editor<'a>(e: &'a Editor, saved: &[(&'a AtUri, &'a str)]) -> Element<'a, Msg> {
    let title = match &e.kind {
        Editing::NewList if e.moderation => "New moderation list",
        Editing::NewList => "New list",
        Editing::NewPack => "New starter pack",
        Editing::List(_) => "Edit list",
        Editing::Pack { .. } => "Edit starter pack",
    };
    let mut col = column![
        row![
            text(title).size(size::TITLE).font(theme::SEMIBOLD),
            space::horizontal(),
            icon_button(Icon::Close, Msg::CloseEditor),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(gap::M);
    if matches!(e.kind, Editing::NewList) {
        col = col.push(ui::muted(if e.moderation {
            "People on a moderation list can be muted or blocked all at once, by you or anyone who subscribes to it."
        } else {
            "A list shows its people's posts as a feed you can pin to Home."
        }));
    }
    if matches!(e.kind, Editing::NewPack) {
        col = col.push(ui::muted(
            "A starter pack suggests people and feeds for others to follow. You're added to it first; add more people from its page.",
        ));
    }
    col = col
        .push(
            column![
                ui::faint("Name"),
                ui::field("Name", &e.name).on_input(Msg::Name),
            ]
            .spacing(gap::XS),
        )
        .push(
            column![
                ui::faint("Description"),
                ui::field("What is it for? (optional)", &e.description).on_input(Msg::Description),
            ]
            .spacing(gap::XS),
        );
    if matches!(e.kind, Editing::Pack { .. }) {
        let mut feeds = Column::new().spacing(gap::XXS);
        for &(u, name) in saved {
            let on = e.feeds.contains(u);
            feeds = feeds.push(
                checkbox(on)
                    .label(name)
                    .on_toggle_maybe(
                        (on || e.feeds.len() < MAX_PACK_FEEDS)
                            .then_some(|_| Msg::ToggleFeed(u.clone())),
                    )
                    .size(size::CHECKBOX)
                    .text_size(size::BODY),
            );
        }
        col = col.push(
            column![
                ui::faint(format!(
                    "Feeds (up to {MAX_PACK_FEEDS}, from your saved feeds)"
                )),
                if saved.is_empty() {
                    Element::from(ui::muted("Save some feeds first to include them here."))
                } else {
                    feeds.into()
                },
            ]
            .spacing(gap::XS),
        );
    }
    if let Some(err) = &e.error {
        col = col.push(text(err).size(size::SMALL).style(theme::danger_text));
    }
    let ok = !e.saving && !e.name.trim().is_empty();
    col = col.push(
        row![
            space::horizontal(),
            ghost("Cancel", Msg::CloseEditor),
            ui::primary(if e.saving { "Saving…" } else { "Save" })
                .on_press_maybe(ok.then_some(Msg::Save)),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center),
    );
    container(col)
        .padding(gap::XL)
        .width(theme::layout::DIALOG_WIDE)
        .style(theme::dialog)
        .into()
}

/// "Add to lists": your lists, ticked where the person is on them.
pub fn add_to(a: &AddTo) -> Element<'_, Msg> {
    let mut col = column![
        row![
            text(format!("Lists for {}", a.name))
                .size(size::TITLE)
                .font(theme::SEMIBOLD),
            space::horizontal(),
            icon_button(Icon::Close, Msg::CloseAddTo),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(gap::M);
    col = col.push(match &a.rows {
        None => status("Loading…"),
        Some(Err(e)) => retry(e.clone()),
        Some(Ok(rows)) if rows.is_empty() => column![
            ui::muted("You haven't made any lists yet."),
            ghost("Make a list", Msg::NewList(false)),
        ]
        .spacing(gap::S)
        .into(),
        Some(Ok(rows)) => {
            let mut c = Column::new().spacing(gap::XS);
            for r in rows {
                let busy = a.busy.contains(&r.list.uri);
                c = c.push(
                    row![
                        checkbox(r.item.is_some())
                            .on_toggle_maybe((!busy).then_some(|_| Msg::Toggle(r.list.uri.clone())))
                            .size(size::CHECKBOX),
                        column![
                            text(&r.list.name).size(size::BODY),
                            ui::faint(r.list.kind_label()).size(size::CAPTION),
                        ]
                        .spacing(gap::XXS),
                    ]
                    .spacing(gap::M)
                    .align_y(Alignment::Center),
                );
            }
            scrollable(c).height(Length::Shrink).into()
        }
    });
    col = col.push(row![
        space::horizontal(),
        ui::primary("Done").on_press(Msg::CloseAddTo)
    ]);
    container(col)
        .padding(gap::XL)
        .width(theme::layout::DIALOG)
        .max_height(560)
        .style(theme::dialog)
        .into()
}
