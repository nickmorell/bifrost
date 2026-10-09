//! The External media card: one switch per service whose links can show as a player
//! card or GIF, as in Bluesky's "External media" settings.

use super::super::message::Msg;
use super::super::props::Prefs;
use super::parts::switch_row;
use crate::components as ui;
use crate::theme::gap;
use iced::Element;
use iced::widget::Column;

pub(super) fn card<M: Clone + 'static>(p: &Prefs, lift: fn(Msg) -> M) -> Element<'static, M> {
    let mut col = Column::new().spacing(gap::L).push(ui::muted(
        "Show links to these services as richer cards. Each is off until you turn it on. \
         Cards may load pictures from the service, which can then see that you looked.",
    ));
    for s in &p.media {
        let id = s.id;
        col = col.push(switch_row(s.name, s.description, s.on, move |on| {
            lift(Msg::Integration(id, on))
        }));
    }
    col.into()
}
