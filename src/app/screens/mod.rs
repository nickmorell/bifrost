//! Top-level layout and the individual screens.
//!
//! The layout follows Zen's habits: a quiet sidebar on the window background, and the
//! current screen on one rounded panel floating beside it.

mod route;
mod sidebar;

use crate::app::interaction;
use crate::app::popups;

use crate::app::{App, Message, Phase};
use crate::components::Icon;
use crate::components::{self as ui, icon_button};
use crate::theme::{self, gap, layout, size};
use iced::widget::{center, column, container, mouse_area, opaque, row, space, stack, text};
use iced::{Alignment, Element, Length, Padding};

impl App {
    pub(super) fn view(&self) -> Element<'_, Message> {
        let screen = match &self.phase {
            Phase::Booting => center(ui::faint("Starting BiFrost…")).into(),
            Phase::Login(state) => state.view().map(Message::Login),
            Phase::Ready => self.main_view(),
        };
        container(screen)
            .style(theme::base)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn main_view(&self) -> Element<'_, Message> {
        let mut panel_layers = if self.deck_active() {
            stack![self.deck_view()]
        } else {
            stack![
                container(self.route_view())
                    .style(theme::panel)
                    .width(Length::Fill)
                    .height(Length::Fill)
            ]
        };
        if let Some(mini) = self.mini_player() {
            panel_layers = panel_layers.push(
                container(mini)
                    .padding(gap::L)
                    .align_bottom(Length::Fill)
                    .align_right(Length::Fill),
            );
        }
        if let Some(t) = &self.toast {
            panel_layers = panel_layers.push(
                container(ui::toast(t, Message::DismissToast))
                    .padding(gap::XL)
                    .align_bottom(Length::Fill)
                    .center_x(Length::Fill),
            );
        }

        let layout = row![self.sidebar(), panel_layers]
            .padding(Padding::from(layout::INSET).left(0.0))
            .height(Length::Fill);

        let mut layers = stack![layout];
        if let Some(c) = &self.composer {
            layers = layers.push(modal(
                self.composer_view(c),
                Message::Composer(super::composer::Msg::Cancel),
            ));
        }
        if let Some(editor) = self.profile_editor() {
            layers = layers.push(modal(
                editor,
                Message::Profile(crate::features::profile::Msg::Editor(
                    crate::features::profile::EditMsg::Cancel,
                )),
            ));
        }
        if let Some(e) = &self.interaction_edit {
            layers = layers.push(modal(
                interaction::dialog(self, e),
                Message::Interaction(interaction::Msg::Close),
            ));
        }
        if let Some(n) = &self.chats.new_chat {
            layers = layers.push(modal(
                self.new_chat_dialog(n),
                Message::Chat(super::chat::Msg::CloseNew),
            ));
        }
        if let Some(s) = &self.chats.share {
            layers = layers.push(modal(
                self.share_dialog(s),
                Message::Chat(super::chat::Msg::CloseShare),
            ));
        }
        if let Some(e) = &self.lists.editor {
            layers = layers.push(modal(
                self.list_editor(e),
                Message::Lists(super::lists::Msg::CloseEditor),
            ));
        }
        if let Some(a) = &self.lists.add_to {
            layers = layers.push(modal(
                Self::add_to_dialog(a),
                Message::Lists(super::lists::Msg::CloseAddTo),
            ));
        }
        if self.deck.adding() {
            layers = layers.push(modal(
                self.add_column_dialog(),
                Message::Deck(super::deck::Msg::CloseAdd),
            ));
        }
        if let Some(dialog) = self.report_dialog() {
            layers = layers.push(modal(
                dialog,
                Message::Moderation(super::moderation::Msg::Cancel),
            ));
        }
        if let Some(content) = &self.popups.select_text {
            layers = layers.push(modal(
                select_text_dialog(content),
                Message::Popup(popups::Msg::CloseSelectText),
            ));
        }
        if let Some((shown, url)) = &self.popups.confirm_link {
            layers = layers.push(modal(
                link_dialog(shown, url),
                Message::Popup(popups::Msg::ConfirmLink(false)),
            ));
        }
        if let Some(v) = &self.viewer {
            layers = layers.push(super::media::viewer_view(self, v));
        }
        if let Some(v) = self.video.as_ref().filter(|v| v.expanded) {
            layers = layers.push(super::video::expanded_view(self, v));
        }
        layers.into()
    }
}

/// A dialog over everything: dimmed backdrop, kept off the window edge, closed by
/// clicking outside it (Esc is handled in `update`).
fn modal(content: Element<'_, Message>, close: Message) -> Element<'_, Message> {
    opaque(mouse_area(center(opaque(content)).padding(gap::XL).style(theme::scrim)).on_press(close))
}

/// Asks before opening a link whose text names another site.
fn link_dialog<'a>(shown: &'a str, url: &'a str) -> Element<'a, Message> {
    let answer = |go| Message::Popup(popups::Msg::ConfirmLink(go));
    container(
        column![
            text("Check this link")
                .size(size::TITLE)
                .font(theme::SEMIBOLD),
            ui::muted(format!(
                "The text says \"{shown}\", but the link opens somewhere else:"
            )),
            text(url).size(size::SMALL).font(theme::SEMIBOLD),
            row![
                space::horizontal(),
                ui::ghost("Cancel", answer(false)),
                ui::primary("Open link").on_press(answer(true)),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        ]
        .spacing(gap::M),
    )
    .padding(gap::XL)
    .max_width(layout::DIALOG)
    .style(theme::dialog)
    .into()
}

/// A post's text in a box where it can be selected and copied.
fn select_text_dialog(content: &iced::widget::text_editor::Content) -> Element<'_, Message> {
    container(
        column![
            row![
                text("Select text").size(size::TITLE).font(theme::SEMIBOLD),
                space::horizontal(),
                icon_button(Icon::Close, Message::Popup(popups::Msg::CloseSelectText)),
            ]
            .align_y(Alignment::Center),
            ui::muted("Drag to select, then Ctrl+C to copy. Ctrl+A selects everything."),
            iced::widget::text_editor(content)
                .on_action(|a| Message::Popup(popups::Msg::SelectTextAction(a)))
                .font(theme::FONT)
                .size(size::BODY)
                .padding(gap::M)
                .height(Length::Shrink)
                .style(theme::editor),
        ]
        .spacing(gap::M),
    )
    .padding(gap::XL)
    .max_width(layout::DIALOG_WIDE)
    .style(theme::dialog)
    .into()
}
