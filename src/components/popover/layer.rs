//! The open menu, laid out over everything else next to what opened it.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Point, Rectangle, Size, mouse};

pub(super) struct Menu<'a, 'b, Message, Theme, Renderer> {
    pub(super) popup: &'b mut Element<'a, Message, Theme, Renderer>,
    pub(super) tree: &'b mut Tree,
    pub(super) anchor: Rectangle,
    /// Open at a point (where a right-click was) rather than beside a widget.
    pub(super) at_point: bool,
    pub(super) on_dismiss: Message,
}

/// Space between the anchor and the menu, and kept clear at the window's edges.
const GAP: f32 = 6.0;

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Menu<'_, '_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds);
        let node = self
            .popup
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);
        let size = node.size();
        let at = place(self.anchor, self.at_point, size, bounds);
        layout::Node::with_children(size, vec![node]).move_to(at)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let Some(content) = layout.children().next() else {
            return;
        };
        self.popup.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            content,
            cursor,
            &layout.bounds(),
        );
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(_)) = event
            && !cursor.is_over(layout.bounds())
        {
            shell.publish(self.on_dismiss.clone());
            shell.capture_event();
            return;
        }
        let Some(content) = layout.children().next() else {
            return;
        };
        self.popup.as_widget_mut().update(
            self.tree,
            event,
            content,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(content) = layout.children().next() else {
            return mouse::Interaction::None;
        };
        self.popup.as_widget().mouse_interaction(
            self.tree,
            content,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        if let Some(content) = layout.children().next() {
            self.popup
                .as_widget_mut()
                .operate(self.tree, content, renderer, operation);
        }
    }
}

/// Where a menu of `size` goes in a window of `bounds`: beside `anchor`, or at it when
/// `at_point`, below unless that runs off the bottom, and never past an edge.
fn place(anchor: Rectangle, at_point: bool, size: Size, bounds: Size) -> Point {
    let x = if at_point {
        // Right of the pointer, or left of it when there is no room.
        if anchor.x + size.width + GAP <= bounds.width {
            anchor.x
        } else {
            anchor.x - size.width
        }
    } else if anchor.center_x() > bounds.width / 2.0 {
        // A button on the right half opens its menu leftwards, like a "…" menu;
        // one on the left half opens rightwards, so the menu sits under the post.
        anchor.x + anchor.width - size.width
    } else {
        anchor.x
    }
    .clamp(GAP, (bounds.width - size.width - GAP).max(GAP));
    let gap = if at_point { 0.0 } else { GAP };
    let below = anchor.y + anchor.height + gap;
    let y = if below + size.height + GAP <= bounds.height {
        below
    } else {
        (anchor.y - size.height - gap).max(GAP)
    };
    Point::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Size = Size::new(800.0, 600.0);
    const MENU: Size = Size::new(200.0, 300.0);

    fn at(x: f32, y: f32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::ZERO)
    }

    #[test]
    fn a_right_click_menu_opens_at_the_pointer() {
        assert_eq!(
            place(at(100.0, 50.0), true, MENU, WINDOW),
            Point::new(100.0, 50.0)
        );
        // Near the right edge it opens leftwards; near the bottom, upwards.
        assert_eq!(
            place(at(700.0, 500.0), true, MENU, WINDOW),
            Point::new(500.0, 200.0)
        );
    }

    #[test]
    fn a_button_menu_opens_beside_its_button() {
        let button = Rectangle::new(Point::new(700.0, 100.0), Size::new(40.0, 20.0));
        assert_eq!(
            place(button, false, MENU, WINDOW),
            Point::new(540.0, 100.0 + 20.0 + GAP)
        );
    }
}
