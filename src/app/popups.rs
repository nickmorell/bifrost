//! A post's menus, hidden content shown on request, copying, and the select-text
//! dialog.

use super::{App, Menu, Message};
use crate::model::AtUri;
use iced::Task;

#[derive(Debug, Clone)]
pub enum Msg {
    Reveal(AtUri),
    /// Open or close a post's menu.
    ToggleMenu(AtUri, Menu),
    CloseMenu,
    /// Put text on the clipboard, saying so.
    CopyText(String),
    /// Opens a post's text where it can be selected and copied.
    SelectText(String),
    SelectTextAction(iced::widget::text_editor::Action),
    CloseSelectText,
    /// The answer to "open this disguised link?": `true` opens it.
    ConfirmLink(bool),
}

/// Menus and small dialogs that are open.
#[derive(Default)]
pub(super) struct Popups {
    /// A post's menu.
    pub(super) menu: Option<(AtUri, Menu)>,
    /// The feed whose filter menu is open.
    pub(super) filter: Option<crate::model::FeedSource>,
    /// A post's text opened for selecting.
    pub(super) select_text: Option<iced::widget::text_editor::Content>,
    /// A disguised link waiting for "Open link": what its text said, and where it goes.
    pub(super) confirm_link: Option<(String, String)>,
}

impl App {
    pub(super) fn popup_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Reveal(uri) => {
                self.revealed.insert(uri);
                Task::none()
            }
            Msg::ToggleMenu(uri, menu) => {
                self.actions.cancel_delete();
                let open = Some((uri, menu));
                self.popups.menu = if self.popups.menu == open { None } else { open };
                Task::none()
            }
            Msg::CloseMenu => {
                self.popups.menu = None;
                self.actions.cancel_delete();
                Task::none()
            }
            Msg::CopyText(text) => {
                self.popups.menu = None;
                self.toast = Some("Copied.".into());
                iced::clipboard::write(text)
            }
            Msg::SelectText(text) => {
                self.popups.menu = None;
                self.popups.select_text =
                    Some(iced::widget::text_editor::Content::with_text(&text));
                Task::none()
            }
            Msg::SelectTextAction(action) => {
                // Selecting and copying only: the text cannot be changed.
                if !action.is_edit()
                    && let Some(content) = &mut self.popups.select_text
                {
                    content.perform(action);
                }
                Task::none()
            }
            Msg::ConfirmLink(go) => match self.popups.confirm_link.take() {
                Some((_, url)) if go => self.handle(Message::Nav(super::nav::Msg::OpenUrl(url))),
                _ => Task::none(),
            },
            Msg::CloseSelectText => {
                self.popups.select_text = None;
                Task::none()
            }
        }
    }
}
