//! What the deck is told, and what it asks of the app.

use crate::model::{DeckColumn, DeckWidth};
use iced::widget::scrollable;

#[derive(Debug, Clone)]
pub enum Msg {
    /// Turns deck mode on or off.
    Toggle(bool),
    Width(DeckWidth),
    OpenAdd,
    CloseAdd,
    Add(DeckColumn),
    Remove(usize),
    /// Moves a column left (-1) or right (1).
    Move(usize, isize),
    Refresh(usize),
    Top(usize),
    ShowNew(usize),
    Scrolled(usize, scrollable::Viewport),
}

/// What the app has to do for the deck.
#[derive(Debug)]
pub enum Event {
    /// Turn deck mode on or off, and save it.
    Toggle(bool),
    /// Save the column width.
    Width(DeckWidth),
    /// Save these as the columns.
    Columns(Vec<DeckColumn>),
    /// Load what a column shows; `true` loads it again even if it is loaded.
    Load(DeckColumn, bool),
    /// Load more of what a column shows.
    More(DeckColumn),
    /// Show the new posts found for a column, or load it again if there are none.
    ShowNew(DeckColumn),
    /// Scroll a column back to its top.
    Top(DeckColumn),
    /// A column was scrolled this far down.
    Scrolled(DeckColumn, f32),
    /// Load your lists, to offer them as columns.
    LoadLists,
    Toast(String),
}
