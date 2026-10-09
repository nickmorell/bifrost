//! A menu that opens over everything else, anchored to the widget that opened it.
//!
//! iced has no pop-up menu of its own; this lays the menu out as an overlay below
//! (or, near the bottom of the window, above) its anchor. A click outside the menu
//! closes it without reaching whatever is underneath.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, mouse};

pub struct Popover<'a, Message, Theme, Renderer> {
    base: Element<'a, Message, Theme, Renderer>,
    popup: Option<Element<'a, Message, Theme, Renderer>>,
    on_dismiss: Message,
}

/// Shows `popup`, if any, over everything else, next to `base`.
pub fn popover<'a, Message, Theme, Renderer>(
    base: impl Into<Element<'a, Message, Theme, Renderer>>,
    popup: Option<Element<'a, Message, Theme, Renderer>>,
    on_dismiss: Message,
) -> Popover<'a, Message, Theme, Renderer> {
    Popover {
        base: base.into(),
        popup,
        on_dismiss,
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Popover<'_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        let mut out = vec![Tree::new(&self.base)];
        out.extend(self.popup.as_ref().map(Tree::new));
        out
    }

    fn diff(&self, tree: &mut Tree) {
        match &self.popup {
            Some(p) => tree.diff_children(&[self.base.as_widget(), p.as_widget()]),
            None => tree.diff_children(&[self.base.as_widget()]),
        }
    }

    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.base.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.base
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
        self.base.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
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
        self.base.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
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
        self.base.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.base
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let (base_tree, rest) = tree.children.split_first_mut()?;
        match (&mut self.popup, rest.first_mut()) {
            (Some(popup), Some(popup_tree)) => Some(overlay::Element::new(Box::new(Menu {
                popup,
                tree: popup_tree,
                anchor: layout.bounds() + translation,
                on_dismiss: self.on_dismiss.clone(),
            }))),
            _ => self.base.as_widget_mut().overlay(
                base_tree,
                layout,
                renderer,
                viewport,
                translation,
            ),
        }
    }
}

struct Menu<'a, 'b, Message, Theme, Renderer> {
    popup: &'b mut Element<'a, Message, Theme, Renderer>,
    tree: &'b mut Tree,
    anchor: Rectangle,
    on_dismiss: Message,
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
        // A button on the right half opens its menu leftwards, like a "…" menu;
        // one on the left half opens rightwards, so the menu sits under the post.
        let x = if self.anchor.center_x() > bounds.width / 2.0 {
            self.anchor.x + self.anchor.width - size.width
        } else {
            self.anchor.x
        }
        .clamp(GAP, (bounds.width - size.width - GAP).max(GAP));
        let below = self.anchor.y + self.anchor.height + GAP;
        let y = if below + size.height + GAP <= bounds.height {
            below
        } else {
            (self.anchor.y - size.height - GAP).max(GAP)
        };
        layout::Node::with_children(size, vec![node]).move_to(Point::new(x, y))
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

impl<'a, Message, Theme, Renderer> From<Popover<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(p: Popover<'a, Message, Theme, Renderer>) -> Self {
        Element::new(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::{button, text};
    use iced_test::simulator::{self, simulator};

    const DISMISS: u8 = 0;
    const COPY: u8 = 1;

    fn menu<'a>(open: bool) -> Element<'a, u8> {
        let popup = open.then(|| button(text("Copy link")).on_press(COPY).into());
        popover(text("Post"), popup, DISMISS).into()
    }

    #[test]
    fn the_menu_shows_only_when_open() {
        assert!(simulator(menu(true)).find("Copy link").is_ok());
        assert!(simulator(menu(false)).find("Copy link").is_err());
    }

    #[test]
    fn a_click_picks_an_item_or_closes_the_menu() {
        let mut ui = simulator(menu(true));
        assert!(ui.click("Copy link").is_ok());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [COPY]);

        let mut ui = simulator(menu(true));
        ui.point_at(Point::new(600.0, 500.0));
        let _ = ui.simulate(simulator::click());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [DISMISS]);
    }
}
