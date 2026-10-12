//! A menu that opens over everything else, anchored to the widget that opened it.
//!
//! iced has no pop-up menu of its own; this lays the menu out as an overlay below
//! (or, near the bottom of the window, above) its anchor. A click outside the menu
//! closes it without reaching whatever is underneath. `context_menu` is the same menu
//! opened by right-clicking, where the pointer was.

mod layer;

use layer::Menu;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector, mouse};

pub struct Popover<'a, Message, Theme, Renderer> {
    base: Element<'a, Message, Theme, Renderer>,
    popup: Option<Element<'a, Message, Theme, Renderer>>,
    on_dismiss: Message,
    /// For a menu opened by right-clicking `base`: what the right-click sends.
    on_right_click: Option<Message>,
}

/// Where the last right-click was, from the top left of the base.
#[derive(Default)]
struct Pointer(Vector);

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
        on_right_click: None,
    }
}

/// Right-clicking `base` sends `on_open`; `popup`, if any, then shows over everything
/// else where the pointer was.
pub fn context_menu<'a, Message, Theme, Renderer>(
    base: impl Into<Element<'a, Message, Theme, Renderer>>,
    popup: Option<Element<'a, Message, Theme, Renderer>>,
    on_open: Message,
    on_dismiss: Message,
) -> Popover<'a, Message, Theme, Renderer> {
    Popover {
        base: base.into(),
        popup,
        on_dismiss,
        on_right_click: Some(on_open),
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Popover<'_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Pointer>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Pointer::default())
    }

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
        if let Some(open) = &self.on_right_click
            && let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event
            && !shell.is_event_captured()
            && let Some(at) = cursor.position_over(layout.bounds())
        {
            tree.state.downcast_mut::<Pointer>().0 = at - layout.position();
            shell.publish(open.clone());
            shell.capture_event();
        }
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
        let at_point = self.on_right_click.is_some();
        let anchor = if at_point {
            let at = layout.position() + tree.state.downcast_ref::<Pointer>().0;
            Rectangle::new(at, Size::ZERO)
        } else {
            layout.bounds()
        } + translation;
        let (base_tree, rest) = tree.children.split_first_mut()?;
        match (&mut self.popup, rest.first_mut()) {
            (Some(popup), Some(popup_tree)) => Some(overlay::Element::new(Box::new(Menu {
                popup,
                tree: popup_tree,
                anchor,
                at_point,
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
    use iced::Point;
    use iced::widget::{button, container, text};
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

    #[test]
    fn a_right_click_opens_the_context_menu() {
        const OPEN: u8 = 2;
        let right_click = |ui: &mut simulator::Simulator<'_, u8>, at: Point| {
            ui.point_at(at);
            let _ = ui.simulate([
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
            ]);
        };
        let post = || {
            context_menu(
                container(text("Post")).width(200).height(100),
                None,
                OPEN,
                DISMISS,
            )
        };
        let mut ui = simulator(post());
        right_click(&mut ui, Point::new(50.0, 50.0));
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [OPEN]);
        // A right-click elsewhere is not on the post.
        let mut ui = simulator(post());
        right_click(&mut ui, Point::new(600.0, 500.0));
        assert!(ui.into_messages().next().is_none());
    }
}
