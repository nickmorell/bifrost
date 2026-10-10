//! What each message from the Settings screen does.

use super::message::{Event, Msg};
use super::state::Screen;
use crate::features::Update;

impl Screen {
    /// Handles one message. Everything but the confirmation is the shell's to apply.
    pub fn update(&mut self, msg: Msg) -> Update<Msg, Event> {
        match msg {
            Msg::Zoom(z) => Update::event(Event::Zoom(z)),
            Msg::Appearance(i) => Update::event(Event::Appearance(i)),
            Msg::SansReading(on) => Update::event(Event::SansReading(on)),
            Msg::ReduceMotion(on) => Update::event(Event::ReduceMotion(on)),
            Msg::Deck(on) => Update::event(Event::Deck(on)),
            Msg::PostLang(code) => Update::event(Event::PostLang(code)),
            Msg::Integration(id, on) => Update::event(Event::Integration(id, on)),
            Msg::Alert(kind, on) => Update::event(Event::Alert(kind, on)),
            Msg::MessageText(on) => Update::event(Event::MessageText(on)),
            Msg::CloseToTray(on) => Update::event(Event::CloseToTray(on)),
            Msg::AltReminder(on) => Update::event(Event::AltReminder(on)),
            Msg::DataSaver(on) => Update::event(Event::DataSaver(on)),
            Msg::Moderation => Update::event(Event::Moderation),
            Msg::CheckUpdates(on) => Update::event(Event::CheckUpdates(on)),
            Msg::ReportProblem => Update::event(Event::ReportProblem),
            Msg::OpenRelease(url) => Update::event(Event::Browse(url)),
            Msg::RemoveAll => {
                self.confirm_remove_all = true;
                Update::none()
            }
            Msg::ConfirmRemoveAll(go) => {
                self.confirm_remove_all = false;
                if go {
                    Update::event(Event::Wipe)
                } else {
                    Update::none()
                }
            }
        }
    }
}
