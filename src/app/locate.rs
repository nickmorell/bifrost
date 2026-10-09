//! Finding where posts are on screen, so the keyboard can move between them.

use iced::Rectangle;
use iced::advanced::widget::operation::{Operation, Outcome, Scrollable};
use iced::advanced::widget::{Id, operate};
use iced::{Task, Vector};

/// Where the posts of a timeline sit inside its scrollable.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    /// Top of each post, measured from the top of the scrolled content.
    pub tops: Vec<Option<f32>>,
    /// How far the content is scrolled, and how tall the visible part is.
    pub offset: f32,
    pub height: f32,
}

/// The container id of the post at `index` in a timeline.
pub fn post_id(index: usize) -> Id {
    Id::from(format!("post-{index}"))
}

/// Measures the first `count` posts inside the scrollable `scroll`.
pub fn posts(scroll: String, count: usize) -> Task<Layout> {
    operate(Locate {
        scroll: Id::from(scroll),
        ids: (0..count).map(post_id).collect(),
        content: None,
        layout: Layout {
            tops: vec![None; count],
            ..Layout::default()
        },
    })
}

struct Locate {
    scroll: Id,
    ids: Vec<Id>,
    /// Top of the scrollable's content, once it has been passed.
    content: Option<f32>,
    layout: Layout,
}

impl Operation<Layout> for Locate {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Layout>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&self.scroll) {
            self.content = Some(content_bounds.y);
            self.layout.offset = translation.y;
            self.layout.height = bounds.height;
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        let (Some(id), Some(top)) = (id, self.content) else {
            return;
        };
        if let Some(i) = self.ids.iter().position(|p| p == id) {
            self.layout.tops[i] = Some(bounds.y - top);
        }
    }

    fn finish(&self) -> Outcome<Layout> {
        Outcome::Some(self.layout.clone())
    }
}

/// The post to move to, `step` posts away from `current`. With nothing picked yet, the
/// first post at the top of the view is picked by `j`, and the one above it by `k`.
pub fn next(layout: &Layout, current: Option<usize>, step: isize) -> Option<usize> {
    let count = layout.tops.len();
    if count == 0 {
        return None;
    }
    if let Some(i) = current.filter(|&i| i < count) {
        Some(i.saturating_add_signed(step).min(count - 1))
    } else {
        let first = layout
            .tops
            .iter()
            .position(|t| t.is_some_and(|y| y >= layout.offset - 1.0))
            .unwrap_or(count - 1);
        Some(if step > 0 {
            first
        } else {
            first.saturating_sub(1)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_between_posts() {
        let layout = Layout {
            tops: vec![Some(0.0), Some(300.0), Some(600.0)],
            offset: 250.0,
            height: 500.0,
        };
        assert_eq!(next(&layout, None, 1), Some(1));
        assert_eq!(next(&layout, None, -1), Some(0));
        assert_eq!(next(&layout, Some(1), 1), Some(2));
        assert_eq!(next(&layout, Some(2), 1), Some(2));
        assert_eq!(next(&layout, Some(0), -1), Some(0));
        assert_eq!(next(&Layout::default(), None, 1), None);
    }
}
