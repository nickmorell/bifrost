//! The playing picture. It moves the video on each time it is drawn and asks for the
//! next redraw when the next frame is due, so the rest of the window isn't rebuilt
//! for every frame.

use iced::advanced::Renderer as _;
use iced::advanced::image::Allocation;
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
    /// The frame due now, which may not be ready to draw yet.
    fn due(&self) -> Option<image::Handle>;
    /// The newest frame ready to draw.
    fn ready(&self) -> Option<image::Handle>;
    /// Draws `frame` from now on: the renderer has it ready.
    fn uploaded(&self, frame: Allocation);
}

struct Moving {
    feed: Arc<dyn Feed>,
    /// Drawn until the first frame is ready.
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
        // A frame is drawn once the renderer has it ready; until then the one
        // before it stays up.
        let ready = self.feed.ready();
        if let Some(due) = self.feed.due()
            && ready.as_ref().map(image::Handle::id) != Some(due.id())
        {
            let feed = self.feed.clone();
            renderer.allocate_image(&due, move |frame| {
                if let Ok(frame) = frame {
                    feed.uploaded(frame);
                }
            });
        }
        if let Some(frame) = ready.as_ref().or(self.still.as_ref()) {
            image::draw(
                renderer,
                layout,
                frame,
                None,
                border::Radius::default(),
                ContentFit::Contain,
                FilterMethod::default(),
                Rotation::default(),
                1.0,
                1.0,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::image::Id;
    use iced_test::simulator;
    use std::sync::Mutex;

    /// A frame due but not ready yet; remembers the frames handed back as ready.
    #[derive(Default)]
    struct Uploading {
        due: Option<image::Handle>,
        ready: Mutex<Vec<Id>>,
    }

    impl Feed for Uploading {
        fn advance(&self) -> Option<Duration> {
            None
        }
        fn due(&self) -> Option<image::Handle> {
            self.due.clone()
        }
        fn ready(&self) -> Option<image::Handle> {
            None
        }
        fn uploaded(&self, frame: Allocation) {
            self.ready.lock().unwrap().push(frame.handle().id());
        }
    }

    #[test]
    fn a_frame_is_drawn_once_the_renderer_has_it_ready() {
        let frame = image::Handle::from_rgba(2, 2, vec![255; 16]);
        let feed = Arc::new(Uploading {
            due: Some(frame.clone()),
            ..Uploading::default()
        });
        let picture: Element<'_, ()> = moving(feed.clone(), None, Length::Fill, Length::Fill);
        let mut ui = simulator(picture);
        // A GPU renderer uploads in the background and hands the frame back on a
        // later draw; a software one hands it back at once.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while feed.ready.lock().unwrap().is_empty() && std::time::Instant::now() < deadline {
            assert!(ui.snapshot(&Theme::Dark).is_ok());
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(feed.ready.lock().unwrap().contains(&frame.id()));
    }
}
