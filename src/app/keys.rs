//! Keyboard shortcuts.

use super::{
    App, Message, Phase, Route, Toggle, actions, chat, compose, composer, deck, interaction, lists,
    moderation, nav, popups, search, video, viewer, window,
};
use crate::model::PostView;
use iced::Task;

impl App {
    pub(super) fn on_key(&mut self, event: iced::keyboard::Event) -> Task<Message> {
        use iced::keyboard::{Event, Key, key::Named};
        let Event::KeyPressed { key, modifiers, .. } = event else {
            return Task::none();
        };
        // Zoom works everywhere, as in a browser.
        if modifiers.command() {
            let zoom = match key.as_ref() {
                Key::Character("=" | "+") => Some(self.settings.zoom_step(1)),
                Key::Character("-") => Some(self.settings.zoom_step(-1)),
                Key::Character("0") => Some(1.0),
                _ => None,
            };
            if let Some(z) = zoom {
                return self.handle(Message::Window(window::Msg::Zoom(z)));
            }
        }
        if self.popups.confirm_link.is_some() {
            return match key.as_ref() {
                Key::Named(Named::Escape) => {
                    self.handle(Message::Popup(popups::Msg::ConfirmLink(false)))
                }
                _ => Task::none(),
            };
        }
        if self.popups.select_text.is_some() {
            return match key.as_ref() {
                Key::Named(Named::Escape) => {
                    self.handle(Message::Popup(popups::Msg::CloseSelectText))
                }
                _ => Task::none(),
            };
        }
        if self.video.as_ref().is_some_and(|v| v.expanded) {
            let m = match key.as_ref() {
                Key::Named(Named::Escape) => video::Msg::Collapse,
                Key::Named(Named::Space) => video::Msg::Toggle,
                Key::Named(Named::ArrowLeft) => video::Msg::Skip(-5.0),
                Key::Named(Named::ArrowRight) => video::Msg::Skip(5.0),
                Key::Character("m") => video::Msg::Mute,
                _ => return Task::none(),
            };
            return self.handle(Message::Video(m));
        }
        if self.viewer.is_some() {
            return match key.as_ref() {
                Key::Named(Named::Escape) => self.handle(Message::Viewer(viewer::Msg::Close)),
                Key::Named(Named::ArrowLeft) => self.handle(Message::Viewer(viewer::Msg::Step(-1))),
                Key::Named(Named::ArrowRight) => self.handle(Message::Viewer(viewer::Msg::Step(1))),
                _ => Task::none(),
            };
        }
        if let (Some(c), Key::Named(Named::Escape)) = (&self.composer, key.as_ref()) {
            let msg = if c.is_confirming_close() {
                composer::Msg::KeepEditing
            } else {
                composer::Msg::Cancel
            };
            return self.handle(Message::Composer(msg));
        }
        if self.profile.editor().is_some() && key == Key::Named(Named::Escape) {
            return self.handle(Message::Profile(crate::features::profile::Msg::Editor(
                crate::features::profile::EditMsg::Cancel,
            )));
        }
        if self.interaction_edit.is_some() && key == Key::Named(Named::Escape) {
            return self.handle(Message::Interaction(interaction::Msg::Close));
        }
        if self.chats.new_chat.is_some() && key == Key::Named(Named::Escape) {
            return self.handle(Message::Chat(chat::Msg::CloseNew));
        }
        if self.chats.share.is_some() && key == Key::Named(Named::Escape) {
            return self.handle(Message::Chat(chat::Msg::CloseShare));
        }
        if self.chats.menu.is_some() && key == Key::Named(Named::Escape) {
            return self.handle(Message::Chat(chat::Msg::Menu(None)));
        }
        if self.lists.editor.is_some() && key == Key::Named(Named::Escape) {
            return self.handle(Message::Lists(lists::Msg::CloseEditor));
        }
        if self.lists.add_to.is_some() && key == Key::Named(Named::Escape) {
            return self.handle(Message::Lists(lists::Msg::CloseAddTo));
        }
        if self.deck.adding() && key == Key::Named(Named::Escape) {
            return self.handle(Message::Deck(deck::Msg::CloseAdd));
        }
        if key == Key::Named(Named::Tab) && !modifiers.command() && !modifiers.alt() {
            // Moves between text boxes; buttons are reached with the shortcuts.
            return if modifiers.shift() {
                iced::widget::operation::focus_previous()
            } else {
                iced::widget::operation::focus_next()
            };
        }
        if self.composer.is_some()
            || self.profile.editor().is_some()
            || self.interaction_edit.is_some()
            || self.chats.new_chat.is_some()
            || self.chats.share.is_some()
            || self.lists.editor.is_some()
            || self.lists.add_to.is_some()
            || self.deck.adding()
            || !matches!(self.phase, Phase::Ready)
        {
            return Task::none();
        }
        if self.moderation.reporting() {
            return match key.as_ref() {
                Key::Named(Named::Escape) => {
                    self.handle(Message::Moderation(moderation::Msg::Cancel))
                }
                _ => Task::none(),
            };
        }
        if modifiers.alt() && key == Key::Named(Named::ArrowLeft) {
            return self.handle(Message::Nav(nav::Msg::Back));
        }
        if modifiers.command() || modifiers.alt() {
            return Task::none();
        }
        let selected = self.selected_post();
        match key.as_ref() {
            Key::Named(Named::Escape) if self.popups.menu.is_some() => {
                self.popups.menu = None;
                self.actions.cancel_delete();
                Task::none()
            }
            Key::Named(Named::Escape) if self.popups.filter.is_some() => {
                self.popups.filter = None;
                Task::none()
            }
            Key::Named(Named::Escape) => self.handle(Message::Nav(nav::Msg::Back)),
            Key::Named(Named::Enter) | Key::Character("o") => match selected {
                Some(p) => self.handle(Message::Nav(nav::Msg::OpenThread(p.uri))),
                None => Task::none(),
            },
            Key::Character("j") => self.handle(Message::Window(window::Msg::Move(1))),
            Key::Character("k") => self.handle(Message::Window(window::Msg::Move(-1))),
            Key::Character("l") => match selected {
                Some(p) => {
                    let toggle = if p.viewer.like.is_some() {
                        Toggle::Off
                    } else {
                        Toggle::On
                    };
                    self.handle(Message::Action(actions::Msg::Like(p.uri, toggle)))
                }
                None => Task::none(),
            },
            Key::Character("n") => self.handle(Message::Compose(compose::Msg::New)),
            Key::Character("a") => match selected {
                Some(p) => self.handle(Message::Nav(nav::Msg::OpenProfile(p.author.did.into()))),
                None => Task::none(),
            },
            Key::Character("b") => match selected {
                Some(p) => {
                    let toggle = if p.viewer.bookmarked {
                        Toggle::Off
                    } else {
                        Toggle::On
                    };
                    self.handle(Message::Action(actions::Msg::Bookmark(p.uri, toggle)))
                }
                None => Task::none(),
            },
            Key::Character("c") => match selected {
                Some(p) => self.handle(Message::Popup(popups::Msg::SelectText(p.record.text))),
                None => Task::none(),
            },
            Key::Character("r") => self.handle(Message::Nav(nav::Msg::Refresh)),
            Key::Character("/") => {
                let go = if self.route() == &Route::Search {
                    Task::none()
                } else {
                    self.handle(Message::Nav(nav::Msg::Navigate(Route::Search)))
                };
                Task::batch([go, iced::widget::operation::focus(search::INPUT)])
            }
            _ => Task::none(),
        }
    }

    /// The post picked with j / k, if the current screen is a timeline.
    pub(super) fn selected_post(&self) -> Option<PostView> {
        let i = self.selected?;
        let key = self.current_timeline_key()?;
        Some(self.timelines.get(&key)?.items().get(i)?.post.clone())
    }
}
