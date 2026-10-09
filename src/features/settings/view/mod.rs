//! The Settings screen. The Translation and Chats cards and the account list belong to
//! other parts of the app, which draw them through [`Host`], so these views build the
//! app's own message type `M` through [`View::lift`].

mod external;
mod notifications;
mod parts;

use super::message::Msg;
use super::props::Prefs;
use super::state::Screen;
use crate::components::{self as ui, eyebrow};
use crate::theme::gap;
use iced::Element;
use iced::widget::{Column, space};

/// What the Settings screen needs from the app around it.
pub trait Host<M> {
    /// The Translation card.
    fn translation(&self) -> Element<'_, M>;
    /// The Privacy card.
    fn privacy(&self) -> Element<'_, M>;
    /// The "Who can message you" card.
    fn chats(&self) -> Element<'_, M>;
    /// The signed-in accounts, with switch, log out and "Add another account".
    fn accounts(&self) -> Element<'_, M>;
}

/// The Settings screen, unscrolled; the shell puts it in its own scrollable.
pub struct View<'a, H, M> {
    pub screen: &'a Screen,
    pub prefs: Prefs,
    pub host: &'a H,
    /// Turns the screen's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<M>, M: Clone + 'static> View<'a, H, M> {
    /// Everything in Settings, top to bottom.
    pub fn body(self) -> Element<'a, M> {
        let (p, lift) = (&self.prefs, self.lift);
        let mut col = Column::new()
            .spacing(gap::M)
            .padding([gap::S, gap::XL])
            .push(eyebrow("Posting"))
            .push(ui::card(parts::posting(p, lift)).padding(gap::L));
        col = section(col, "Privacy").push(ui::card(self.host.privacy()).padding(gap::L));
        col = section(col, "Translation").push(ui::card(self.host.translation()).padding(gap::L));
        col = section(col, "Appearance").push(ui::card(parts::appearance(p, lift)).padding(gap::L));
        col = section(col, "Layout").push(ui::card(parts::layout(p, lift)).padding(gap::L));
        col = section(col, "Accessibility")
            .push(ui::card(parts::accessibility(p, lift)).padding(gap::L));
        col = section(col, "Chats").push(ui::card(self.host.chats()).padding(gap::L));
        col = section(col, "Notifications")
            .push(ui::card(notifications::card(p, lift)).padding(gap::L));
        col = section(col, "Moderation")
            .push(ui::card(parts::moderation(p.muted_words, lift)).padding(gap::XS));
        col =
            section(col, "External media").push(ui::card(external::card(p, lift)).padding(gap::L));
        if !p.services.is_empty() {
            col = parts::integrations(section(col, "Integrations"), p, lift);
        }
        col = section(col, "Accounts")
            .push(self.host.accounts())
            .push(parts::remove_all(self.screen.confirming(), lift));
        section(col, "Keyboard shortcuts")
            .push(ui::card(parts::shortcuts()).padding(gap::L))
            .into()
    }
}

/// A gap and a heading above the next card.
fn section<'a, M: 'a>(col: Column<'a, M>, heading: &'static str) -> Column<'a, M> {
    col.push(space::vertical().height(gap::L))
        .push(eyebrow(heading))
}
