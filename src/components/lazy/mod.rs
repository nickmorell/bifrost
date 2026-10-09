//! A long list that only draws the rows near the scroll position. Rows further away
//! are replaced by space of the height they were last drawn at (or a guess), so a
//! list of hundreds of posts costs about as much to draw as a screenful.
//!
//! The list doesn't scroll by itself: it goes inside the screen's scrollable, which
//! reports its offset. Each drawn row reports its height through `measured`; the
//! caller keeps those in [`Heights`] and hands them back next time.

use iced::Element;
use iced::widget::{Column, keyed, sensor, space};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[cfg(test)]
mod tests;

/// How tall each row was last drawn, by its key.
#[derive(Debug, Clone, Default)]
pub struct Heights(HashMap<u64, f32>);

impl Heights {
    pub fn get(&self, key: u64) -> Option<f32> {
        self.0.get(&key).copied()
    }

    pub fn set(&mut self, key: u64, height: f32) {
        self.0.insert(key, height);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Forgets the rows `keep` says no to.
    pub fn retain(&mut self, keep: impl Fn(u64) -> bool) {
        self.0.retain(|k, _| keep(*k));
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A row's key: anything that names it, such as a post's address.
pub fn row_key(id: &(impl Hash + ?Sized)) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut h);
    h.finish()
}

/// The part of the list on screen.
#[derive(Debug, Clone, Copy)]
pub struct Window {
    /// How far the list is scrolled.
    pub offset: f32,
    /// How tall the screen is.
    pub screen: f32,
}

impl Window {
    /// Rows within two screens above and three below are drawn, so fast scrolling
    /// rarely reaches an edge before the next frame.
    fn range(self) -> (f32, f32) {
        (
            self.offset - 2.0 * self.screen,
            self.offset + 3.0 * self.screen,
        )
    }
}

/// The rows `rows` gives as (key, data), drawing only those near `window` with `draw`
/// (given each row's index and data). `guess` is the height of a row never drawn.
pub fn lazy_list<'a, D, M: 'a>(
    rows: impl IntoIterator<Item = (u64, D)>,
    heights: &Heights,
    guess: f32,
    window: Window,
    mut draw: impl FnMut(usize, D) -> Element<'a, M>,
    measured: impl Fn(u64, f32) -> M + Clone + 'a,
) -> Element<'a, M> {
    let (from, to) = window.range();
    let (mut top, mut above, mut below) = (0.0, 0.0, 0.0);
    let mut drawn = keyed::Column::new();
    for (i, (key, data)) in rows.into_iter().enumerate() {
        let height = heights.get(key).unwrap_or(guess);
        let shown = top + height >= from && top <= to;
        top += height;
        if !shown {
            if top <= from {
                above += height;
            } else {
                below += height;
            }
            continue;
        }
        let measured = measured.clone();
        drawn = drawn.push(
            key,
            sensor(draw(i, data)).on_resize(move |s| measured(key, s.height)),
        );
    }
    // A plain column around the keyed one on purpose: iced's keyed column hands a
    // child the old state without checking its widget type, which panics when the
    // list is swapped for another.
    Column::new()
        .push(space::vertical().height(above))
        .push(drawn)
        .push(space::vertical().height(below))
        .into()
}
