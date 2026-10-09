//! The app's side of moderation (`features::moderation`): passing it messages, doing
//! what its events ask, and placing the screen and the report dialog.

use super::{App, Message, nav, prefs};
use crate::features::moderation::{Event, Screen};

pub use crate::features::moderation::{Moderation, Msg};
use iced::{Element, Task};

impl App {
    pub(super) fn moderation_update(&mut self, msg: Msg) -> Task<Message> {
        if let Msg::Report(_) = msg {
            self.popups.menu = None;
        }
        let out = self.moderation.update(msg, &self.client);
        let mut tasks = vec![out.task.map(Message::Moderation)];
        for event in out.events {
            tasks.push(match event {
                Event::Edit(edit) => self.handle(prefs::edit(edit)),
                Event::People(kind) => self.handle(Message::Nav(nav::Msg::OpenPeople(
                    self.my_did.to_string(),
                    kind,
                ))),
                Event::Toast(t) => {
                    self.toast = Some(t);
                    Task::none()
                }
                Event::Failed(e) => self.handle_error(e),
            });
        }
        Task::batch(tasks)
    }

    /// The moderation screen.
    pub(super) fn moderation_view(&self) -> Element<'_, Message> {
        let screen = Screen {
            moderation: &self.moderation,
            prefs: &self.prefs,
        };
        self.scroll(screen.view().map(Message::Moderation))
    }

    /// The report dialog, while a report is being written.
    pub(super) fn report_dialog(&self) -> Option<Element<'_, Message>> {
        Some(self.moderation.dialog()?.map(Message::Moderation))
    }
}
