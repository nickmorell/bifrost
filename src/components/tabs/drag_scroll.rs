//! A wrapper that makes a horizontal [`Scrollable`](iced::widget::Scrollable) easier to
//! move: the mouse wheel scrolls it sideways, and it can be dragged with the mouse
//! from anywhere on its content, like a touch list.
//!
//! A press that moves less than [`DRAG_THRESHOLD`] is left alone, so the buttons
//! inside still receive ordinary clicks.

use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::operation::scrollable::{AbsoluteOffset, scroll_by};
use iced::advanced::widget::{self, Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector};

/// How far the pointer must travel before a press becomes a drag.
const DRAG_THRESHOLD: f32 = 4.0;
/// Pixels moved per wheel line.
const LINE: f32 = 60.0;

pub struct DragScroll<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    target: widget::Id,
    /// Height of the strip along the bottom that belongs to the scrollbar, where
    /// presses are passed straight through.
    rail: f32,
}

/// Wraps `content`, a horizontal scrollable with the id `target`.
pub fn drag_scroll<'a, Message, Theme, Renderer>(
    target: impl Into<widget::Id>,
    rail: f32,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> DragScroll<'a, Message, Theme, Renderer> {
    DragScroll {
        content: content.into(),
        target: target.into(),
        rail,
    }
}

#[derive(Default)]
struct State {
    /// Where the press started and the last pointer x seen, while the button is down.
    press: Option<(Point, f32)>,
    dragging: bool,
}

impl<Message, Theme, Renderer> DragScroll<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn scroll(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, dx: f32) {
        let mut op = scroll_by(self.target.clone(), AbsoluteOffset { x: dx, y: 0.0 });
        let op: &mut dyn Operation = &mut op;
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, op);
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for DragScroll<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let mut forward_cursor = cursor;

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                // Use whichever axis the wheel or trackpad moved most.
                let (x, y) = match *delta {
                    mouse::ScrollDelta::Lines { x, y } => (x * LINE, y * LINE),
                    mouse::ScrollDelta::Pixels { x, y } => (x, y),
                };
                let d = if x.abs() > y.abs() { x } else { y };
                if d != 0.0 {
                    self.scroll(tree, layout, renderer, -d);
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let state: &mut State = tree.state.downcast_mut();
                state.dragging = false;
                state.press = cursor
                    .position_over(bounds)
                    .filter(|p| p.y < bounds.y + bounds.height - self.rail)
                    .map(|p| (p, p.x));
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let state: &mut State = tree.state.downcast_mut();
                if let Some((start, last_x)) = state.press {
                    if !state.dragging && (position.x - start.x).abs() > DRAG_THRESHOLD {
                        state.dragging = true;
                    }
                    if state.dragging {
                        let dx = last_x - position.x;
                        state.press = Some((start, position.x));
                        self.scroll(tree, layout, renderer, dx);
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let state: &mut State = tree.state.downcast_mut();
                let was_dragging = state.dragging;
                state.press = None;
                state.dragging = false;
                if was_dragging {
                    // Let the pressed button reset without treating this as a click.
                    forward_cursor = mouse::Cursor::Unavailable;
                    shell.capture_event();
                }
            }
            _ => {}
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            forward_cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<State>().dragging {
            return mouse::Interaction::Grabbing;
        }
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<DragScroll<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(w: DragScroll<'a, Message, Theme, Renderer>) -> Self {
        Element::new(w)
    }
}
