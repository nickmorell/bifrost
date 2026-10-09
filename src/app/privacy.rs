//! The Privacy card in Settings: the `features::privacy` switches, and who can
//! reply to and quote new posts.

use super::{App, Message, interaction, prefs};
use crate::components as ui;
use crate::features::privacy::Event;
use crate::model::PrefEdit;
use crate::theme::gap;
use iced::widget::{column, text};
use iced::{Element, Task};

pub use crate::features::privacy::{Msg, Privacy};

impl App {
    pub(super) fn privacy_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.privacy.update(msg, &self.client);
        let mut tasks = vec![out.task.map(Message::Privacy)];
        for Event::Failed(e) in out.events {
            tasks.push(self.action_failed("change your privacy settings", e));
        }
        Task::batch(tasks)
    }

    pub(super) fn privacy_settings(&self) -> Element<'_, Message> {
        column![
            self.privacy.view().map(Message::Privacy),
            ui::divider(),
            text("Who can reply to and quote new posts").size(crate::theme::size::BODY),
            interaction::panel(self, &self.prefs.interaction, true, &|i| {
                prefs::edit(PrefEdit::PostInteraction(i))
            }),
        ]
        .spacing(gap::L)
        .into()
    }
}
