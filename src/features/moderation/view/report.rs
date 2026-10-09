//! The dialog for reporting a post or an account.

use crate::api::{ReportReason, ReportSubject};
use crate::components::{self as ui, Icon, ghost, icon_button};
use crate::features::moderation::{Moderation, Msg};
use crate::theme::{self, gap, layout, size};
use iced::widget::{Column, button, column, container, row, space, text};
use iced::{Alignment, Element, Length};

impl Moderation {
    /// The report dialog, while a report is being written.
    pub fn dialog(&self) -> Option<Element<'_, Msg>> {
        let r = self.report.as_ref()?;
        let what = match &r.subject {
            ReportSubject::Post(_) => "Report this post",
            ReportSubject::Account(_) => "Report this account",
        };
        let mut reasons = Column::new().spacing(gap::XXS);
        for reason in ReportReason::ALL {
            reasons = reasons.push(
                button(text(reason.to_string()).size(size::BODY))
                    .width(Length::Fill)
                    .padding([gap::S, gap::M])
                    .style(theme::nav(r.reason == Some(reason)))
                    .on_press(Msg::Reason(reason)),
            );
        }
        let send = ui::primary(if r.sending {
            "Sending…"
        } else {
            "Send report"
        })
        .on_press_maybe((r.reason.is_some() && !r.sending).then_some(Msg::Submit));
        let dialog = container(
            column![
                row![
                    text(what).size(size::TITLE).font(theme::SEMIBOLD),
                    space::horizontal(),
                    icon_button(Icon::Close, Msg::Cancel),
                ]
                .align_y(Alignment::Center),
                ui::muted("Reports go to Bluesky's moderation team."),
                reasons,
                ui::field(
                    "Anything else moderators should know (optional)",
                    &r.details
                )
                .on_input(Msg::Details),
                row![space::horizontal(), ghost("Cancel", Msg::Cancel), send]
                    .spacing(gap::S)
                    .align_y(Alignment::Center),
            ]
            .spacing(gap::M),
        )
        .padding(gap::XL)
        .width(layout::DIALOG)
        .style(theme::dialog);
        Some(dialog.into())
    }
}
