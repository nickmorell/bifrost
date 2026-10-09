//! Adding, removing and moving columns, and what the column buttons ask for.

use super::message::{Event, Msg};
use super::state::{Deck, MAX_COLUMNS};
use crate::features::Update;
use crate::model::DeckColumn;

type Out = Update<Msg, Event>;

impl Deck {
    /// Handles one message, given the columns shown now.
    pub fn update(&mut self, msg: Msg, columns: &[DeckColumn]) -> Out {
        let column = |i: usize| columns.get(i).cloned();
        match msg {
            Msg::Toggle(on) => Update::event(Event::Toggle(on)),
            Msg::Width(w) => Update::event(Event::Width(w)),
            Msg::OpenAdd => {
                self.adding = true;
                Update::event(Event::LoadLists)
            }
            Msg::CloseAdd => {
                self.adding = false;
                Update::none()
            }
            Msg::Add(c) => {
                self.adding = false;
                if columns.contains(&c) {
                    return Update::event(Event::Toast(format!(
                        "{} is already a column.",
                        c.title()
                    )));
                }
                if columns.len() >= MAX_COLUMNS {
                    return Update::event(Event::Toast(format!(
                        "A deck holds up to {MAX_COLUMNS} columns."
                    )));
                }
                let toast = format!("Added {} to your columns.", c.title());
                let mut cols = columns.to_vec();
                cols.push(c.clone());
                Update::event(Event::Toast(toast))
                    .and(Event::Columns(cols))
                    .and(Event::Load(c, false))
            }
            Msg::Remove(i) => {
                if i >= columns.len() {
                    return Update::none();
                }
                let mut cols = columns.to_vec();
                cols.remove(i);
                Update::event(Event::Columns(cols))
            }
            Msg::Move(i, step) => {
                let j = i.checked_add_signed(step).filter(|j| *j < columns.len());
                match j {
                    Some(j) if i < columns.len() => {
                        let mut cols = columns.to_vec();
                        cols.swap(i, j);
                        Update::event(Event::Columns(cols))
                    }
                    _ => Update::none(),
                }
            }
            Msg::Refresh(i) => {
                column(i).map_or_else(Update::none, |c| Update::event(Event::Load(c, true)))
            }
            Msg::Top(i) => column(i).map_or_else(Update::none, |c| Update::event(Event::Top(c))),
            Msg::ShowNew(i) => {
                column(i).map_or_else(Update::none, |c| Update::event(Event::ShowNew(c)))
            }
            Msg::Scrolled(i, v) => {
                let Some(c) = column(i) else {
                    return Update::none();
                };
                let out = Update::event(Event::Scrolled(c.clone(), v.absolute_offset().y));
                if v.relative_offset().y > 0.85 {
                    out.and(Event::More(c))
                } else {
                    out
                }
            }
        }
    }
}
