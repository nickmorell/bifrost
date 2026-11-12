//! Which speakers or headphones sound plays through.

use super::super::message::Msg;
use super::super::props::Speaker;
use super::parts::titled;
use crate::components::select;
use crate::theme::gap;
use iced::widget::row;
use iced::{Alignment, Element, Length};

/// Picks the device. `chosen` is the saved device's id, `None` for the system's
/// default; a saved device that isn't plugged in shows as such.
pub(super) fn card<'a, M: Clone + 'static>(
    speakers: &'a [Speaker],
    chosen: Option<&str>,
    lift: fn(Msg) -> M,
) -> Element<'a, M> {
    let (selected, placeholder) = shown(speakers, chosen);
    row![
        titled(
            "Play sound through",
            "Videos, live streams and music play on this device. When it isn't plugged in, sound goes to your system's default.",
        )
        .width(Length::Fill),
        select(speakers, selected, move |s: Speaker| lift(Msg::Speaker(s.id)))
            .placeholder(placeholder)
            .width(220),
    ]
    .spacing(gap::L)
    .align_y(Alignment::Center)
    .into()
}

/// The device shown as picked, and what shows when none of the list is: a saved
/// device that isn't plugged in, or the default before the list has come.
pub(in super::super) fn shown(
    speakers: &[Speaker],
    chosen: Option<&str>,
) -> (Option<Speaker>, &'static str) {
    let selected = speakers.iter().find(|s| s.id.as_deref() == chosen).cloned();
    let placeholder = if chosen.is_some() && !speakers.is_empty() {
        "Not plugged in"
    } else {
        "System default"
    };
    (selected, placeholder)
}
