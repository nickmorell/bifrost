//! A body with a footer under it that always stays in view.
//!
//! In a column the body would take all the room there is and leave the footer none
//! once the body is long. This lays the footer out first and gives the body what is
//! left, so a body that scrolls shrinks instead of pushing the footer away.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Point, Rectangle, Size, Vector, mouse};

pub struct Footed<'a, Message, Theme, Renderer> {
    /// The body, then the footer.
    children: [Element<'a, Message, Theme, Renderer>; 2],
}

/// `body` with `footer` under it. The footer always gets the room it needs; the body
/// gets the rest, so it should scroll when it can be long.
pub fn footed<'a, Message, Theme, Renderer>(
    body: impl Into<Element<'a, Message, Theme, Renderer>>,
    footer: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Footed<'a, Message, Theme, Renderer> {
    Footed {
        children: [body.into(), footer.into()],
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Footed<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        let [body, footer] = &mut self.children;
        let [body_tree, footer_tree] = &mut tree.children[..] else {
            return layout::Node::new(Size::ZERO);
        };
        let footer_node = footer.as_widget_mut().layout(
            footer_tree,
            renderer,
            &layout::Limits::new(Size::ZERO, max),
        );
        let footer_height = footer_node.size().height;
        let room = Size::new(max.width, (max.height - footer_height).max(0.0));
        let body_node = body.as_widget_mut().layout(
            body_tree,
            renderer,
            &layout::Limits::new(Size::ZERO, room),
        );
        let body_height = body_node.size().height;
        let size = limits.resolve(
            Length::Fill,
            Length::Shrink,
            Size::new(max.width, body_height + footer_height),
        );
        layout::Node::with_children(
            size,
            vec![body_node, footer_node.move_to(Point::new(0.0, body_height))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((child, state), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
                    .operate(state, layout, renderer, operation);
            }
        });
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
        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
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
        for ((child, state), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(state, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Footed<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(f: Footed<'a, Message, Theme, Renderer>) -> Self {
        Element::new(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::{button, column, container, scrollable, text};
    use iced_test::simulator;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Post,
    }

    #[test]
    fn the_footer_stays_in_view_under_a_long_body() {
        let lines = (0..200).map(|i| Element::<Msg>::from(text(format!("Line {i}"))));
        let body = scrollable(iced::widget::Column::with_children(lines));
        let view =
            container(column![footed(body, button("Post").on_press(Msg::Post))]).max_height(300);
        let mut ui = simulator(view);
        let post = ui.find("Post").unwrap();
        assert!(post.bounds().y + post.bounds().height <= 300.0, "{post:?}");
        assert!(ui.click("Post").is_ok());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Msg::Post]);
    }
}
