//! The Notifications card: which kinds of activity pop up from the system, and
//! whether closing the window keeps BiFrost in the tray.

use super::super::message::Msg;
use super::super::props::Prefs;
use super::parts::switch_row;
use crate::components as ui;
use crate::model::AlertKind;
use crate::theme::gap;
use iced::Element;
use iced::widget::Column;

/// The name and explanation of each kind's switch.
fn words(kind: AlertKind) -> (&'static str, &'static str) {
    match kind {
        AlertKind::Mentions => (
            "Mentions and quotes",
            "When someone mentions you or quotes your post.",
        ),
        AlertKind::Replies => ("Replies", "When someone replies to you."),
        AlertKind::Likes => ("Likes", "When someone likes your post."),
        AlertKind::Reposts => ("Reposts", "When someone reposts your post."),
        AlertKind::Follows => ("New followers", "When someone follows you."),
        AlertKind::Messages => ("Direct messages", "When someone sends you a message."),
    }
}

pub(super) fn card<M: Clone + 'static>(p: &Prefs, lift: fn(Msg) -> M) -> Element<'static, M> {
    let mut col = Column::new().spacing(gap::L).push(ui::muted(
        "Your system shows these while BiFrost is in the background.",
    ));
    for kind in AlertKind::ALL {
        let (title, about) = words(kind);
        col = col.push(switch_row(title, about, p.alerts.is_on(kind), move |on| {
            lift(Msg::Alert(kind, on))
        }));
    }
    if p.alerts.messages {
        col = col.push(switch_row(
            "Show message text",
            "Direct message notices show what was said, not just who sent it. Anyone who can see your screen can read them.",
            p.alerts.message_text,
            move |on| lift(Msg::MessageText(on)),
        ));
    }
    if let Some(on) = p.close_to_tray {
        col = col.push(switch_row(
            "Keep running in the tray",
            "Closing the window keeps BiFrost in the system tray, so notifications still arrive. Quit from the tray icon's menu.",
            on,
            move |on| lift(Msg::CloseToTray(on)),
        ));
    }
    col.into()
}
