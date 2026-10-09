//! The app's side of lists of accounts (`features::people`): passing them messages,
//! doing what their events ask, and placing them.

use super::{App, Message, Toggle, account, actions, nav};
use crate::features::people::{Event, View};

pub use crate::features::people::{Msg, People};
use crate::model::{Author, PeopleKind};
use iced::{Element, Task};

impl App {
    pub(super) fn people_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.people.update(msg, &self.client);
        let mut tasks = vec![out.task.map(Message::People)];
        for event in out.events {
            tasks.push(match event {
                Event::Pictures(urls) => self.request_images(urls),
                Event::Failed(e) => self.handle_session_error(e),
                Event::OpenProfile(did) => {
                    self.handle(Message::Nav(nav::Msg::OpenProfile(did.to_string())))
                }
                Event::Mute(did, on) => {
                    self.handle(Message::Action(actions::Msg::Mute(did, toggle(on))))
                }
                Event::Block(did, on) => {
                    self.handle(Message::Action(actions::Msg::Block(did, toggle(on))))
                }
                Event::SignInAgain => self.handle(Message::Account(account::Msg::Changed(Ok(())))),
            });
        }
        Task::batch(tasks)
    }

    /// The list `kind` of `actor`, as a screen.
    pub(super) fn people_view<'a>(
        &'a self,
        actor: &'a str,
        kind: PeopleKind,
    ) -> Element<'a, Message> {
        let list = View {
            people: &self.people,
            actor,
            kind,
            pictures: self,
        };
        self.scroll(list.view().map(Message::People))
    }

    /// One account; clicking it opens the profile.
    pub(super) fn person_row<'a>(&self, a: &'a Author) -> Element<'a, Message> {
        crate::features::people::person(self, a).map(Message::People)
    }
}

fn toggle(on: bool) -> Toggle {
    if on { Toggle::On } else { Toggle::Off }
}
