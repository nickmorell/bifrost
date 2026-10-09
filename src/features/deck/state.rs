//! The deck's own state, and the columns a new deck starts with.

use crate::model::{DeckColumn, FeedSource};

/// Most columns a deck holds.
pub const MAX_COLUMNS: usize = 8;

#[derive(Debug, Default)]
pub struct Deck {
    /// The "Add a column" dialog is open.
    pub(super) adding: bool,
}

impl Deck {
    pub fn adding(&self) -> bool {
        self.adding
    }
}

/// The columns of a deck not set up yet: Following, notifications and the first
/// pinned feeds, four at most.
pub fn starting<'a>(
    pinned: impl IntoIterator<Item = (&'a FeedSource, &'a str)>,
) -> Vec<DeckColumn> {
    let mut cols = vec![DeckColumn::Following, DeckColumn::Notifications];
    for (source, name) in pinned {
        if cols.len() >= 4 {
            break;
        }
        let c = DeckColumn::for_source(source, name);
        if !cols.contains(&c) {
            cols.push(c);
        }
    }
    cols
}

/// Names a column's scroll position. Stays the same when columns move.
pub fn scroll_id(column: &DeckColumn) -> String {
    format!("deck:{column:?}")
}
