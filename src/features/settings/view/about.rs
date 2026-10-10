//! The About card: which version this is, the opt-in update check, and reporting a
//! problem on GitHub.

use super::super::message::Msg;
use super::super::props::{Prefs, UpdateCheck};
use super::parts::{switch_row, titled};
use crate::components::{self as ui, ghost};
use crate::theme::{self, gap, size};
use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length};

pub(super) fn card<M: Clone + 'static>(p: &Prefs, lift: fn(Msg) -> M) -> Element<'static, M> {
    let mut col = column![
        column![
            text(format!("BiFrost {}", p.version))
                .size(size::BODY)
                .font(theme::MEDIUM),
            ui::muted("Free and open source, under the Apache 2.0 licence."),
        ]
        .spacing(gap::XS),
        switch_row(
            "Check for updates",
            "When BiFrost starts, it asks GitHub whether a newer version is out. GitHub sees your IP address and nothing else. BiFrost never updates itself.",
            p.update != UpdateCheck::Off,
            move |on| lift(Msg::CheckUpdates(on)),
        ),
    ]
    .spacing(gap::L);
    if let Some(found) = status(&p.update, lift) {
        col = col.push(found);
    }
    col.push(
        row![
            titled(
                "Report a problem",
                "Opens the bug form on GitHub in your browser, with your BiFrost version filled in. Nothing is sent until you submit it there.",
            )
            .width(Length::Fill),
            ui::secondary("Report…").on_press(lift(Msg::ReportProblem)),
        ]
        .spacing(gap::L)
        .align_y(Alignment::Center),
    )
    .into()
}

/// What the update check found, under its switch.
fn status<M: Clone + 'static>(
    check: &UpdateCheck,
    lift: fn(Msg) -> M,
) -> Option<Element<'static, M>> {
    Some(match check {
        UpdateCheck::Off => return None,
        UpdateCheck::Checking => ui::muted("Checking…").into(),
        UpdateCheck::UpToDate => ui::muted("You have the newest version.").into(),
        UpdateCheck::Failed(why) => ui::muted(format!("Couldn't check for updates: {why}")).into(),
        UpdateCheck::Available { version, url } => row![
            text(format!("BiFrost {version} is out."))
                .size(size::BODY)
                .font(theme::MEDIUM)
                .width(Length::Fill),
            ghost("Download", lift(Msg::OpenRelease(url.clone()))),
        ]
        .spacing(gap::L)
        .align_y(Alignment::Center)
        .into(),
    })
}
