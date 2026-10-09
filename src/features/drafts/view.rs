//! The list of drafts.

use super::Msg;
use crate::components::{self as ui, full_time, ghost, relative_time, status};
use crate::model::Draft;
use crate::theme::{gap, size};
use iced::widget::{Column, column, row, text};
use iced::{Alignment, Element, Length};

/// Your drafts, unscrolled.
pub struct View<'a> {
    pub drafts: &'a [Draft],
}

impl<'a> View<'a> {
    pub fn view(self) -> Element<'a, Msg> {
        if self.drafts.is_empty() {
            return status("No drafts. Anything you write and leave unsent is kept here.");
        }
        let mut col = Column::new().spacing(gap::S).padding([0.0, gap::L]);
        for d in self.drafts {
            col = col.push(card(d));
        }
        col.into()
    }
}

fn card(d: &Draft) -> Element<'_, Msg> {
    let mut meta = vec![relative_time(&d.saved_at)];
    if d.reply_to.is_some() {
        meta.push("Reply".into());
    } else if d.quote.is_some() {
        meta.push("Quote".into());
    }
    match d.images.len() {
        0 => {}
        1 => meta.push("1 image".into()),
        n => meta.push(format!("{n} images")),
    }
    if let Some(at) = &d.send_at {
        meta.push(format!("Scheduled for {}", full_time(at)));
    }
    let preview = if d.text.trim().is_empty() {
        "(no text)".to_string()
    } else {
        ui::truncate(d.text.trim(), 240)
    };
    ui::card(
        column![
            text(preview).size(size::BODY).line_height(1.5),
            row![ui::faint(meta.join(" · ")).width(Length::Fill),]
                .push(
                    d.send_at
                        .is_some()
                        .then(|| ghost("Unschedule", Msg::Unschedule(d.id)))
                )
                .push(ghost("Delete", Msg::Delete(d.id)))
                .push(ui::secondary("Open").on_press(Msg::Open(d.id)))
                .spacing(gap::S)
                .align_y(Alignment::Center),
        ]
        .spacing(gap::M),
    )
    .padding([gap::M, gap::L])
    .into()
}
