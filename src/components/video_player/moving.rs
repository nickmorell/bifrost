//! The playing picture. It moves the video on each time it is drawn and asks for the
//! next redraw when the next frame is due, so the rest of the window isn't rebuilt
//! for every frame.

use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer};
use iced::widget::image::{self, FilterMethod};
use iced::{ContentFit, Element, Event, Length, Rectangle, Rotation, Size, Theme, border, window};
use std::sync::Arc;
use std::time::Duration;

/// Where the frames come from: a playing video.
pub trait Feed: Send + Sync {
    /// Moves on to the frame due now, and says how long until the next one, or
    /// `None` when nothing changes without input.
    fn advance(&self) -> Option<Duration>;
    /// The frame on screen and the one before it.
    fn frame(&self) -> Option<(image::Handle, Option<image::Handle>)>;
}

struct Moving {
    feed: Arc<dyn Feed>,
    /// Drawn underneath until a second frame has come.
    still: Option<image::Handle>,
    width: Length,
    height: Length,
}

/// The video's picture, fitted inside `w` by `h`.
pub(super) fn moving<'a, M: 'a>(
    feed: Arc<dyn Feed>,
    still: Option<image::Handle>,
    width: Length,
    height: Length,
) -> Element<'a, M> {
    Element::new(Moving {
        feed,
        still,
        width,
        height,
    })
}

impl<M> Widget<M, Theme, iced::Renderer> for Moving {
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(self.width, self.height, Size::ZERO))
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = event
            && layout.bounds().intersects(viewport)
            && let Some(next) = self.feed.advance()
        {
            shell.request_redraw_at(*now + next);
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let Some((frame, previous)) = self.feed.frame() else {
            return;
        };
        let mut draw = |h: &image::Handle| {
            image::draw(
                renderer,
                layout,
                h,
                None,
                border::Radius::default(),
                ContentFit::Contain,
                FilterMethod::default(),
                Rotation::default(),
                1.0,
                1.0,
            );
        };
        if let Some(under) = previous.as_ref().or(self.still.as_ref()) {
            draw(under);
        }
        draw(&frame);
    }
}
