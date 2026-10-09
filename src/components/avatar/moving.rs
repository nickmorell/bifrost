//! Pictures that move on their own: an animated GIF and a loading placeholder's
//! pulse. Each asks the window to redraw when its next change is due, and only while
//! it is on screen, so nothing else is rebuilt for every frame.

use crate::theme;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer};
use iced::widget::image::{self, FilterMethod};
use iced::{ContentFit, Element, Event, Length, Rectangle, Rotation, Size, Theme, border, window};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The frames of an animated GIF, each with how long it stays up.
#[derive(Debug, Clone)]
pub struct Frames(pub Arc<Vec<(image::Handle, Duration)>>);

impl Frames {
    /// The frame to show `elapsed` after the animation started, looping, and how long
    /// until it changes.
    pub fn at(&self, elapsed: Duration) -> (&image::Handle, Duration) {
        let total: Duration = self.0.iter().map(|(_, d)| *d).sum();
        let mut t = Duration::from_nanos(
            (elapsed.as_nanos() % total.as_nanos().max(1)).min(u128::from(u64::MAX)) as u64,
        );
        for (h, d) in self.0.iter() {
            if t < *d {
                return (h, d.saturating_sub(t));
            }
            t -= *d;
        }
        (&self.0[0].0, self.0[0].1)
    }

    /// The first frame, for when motion is reduced.
    pub fn first(&self) -> Option<&image::Handle> {
        self.0.first().map(|(h, _)| h)
    }
}

/// Shortest time between redraws of a moving picture, whatever its frames say.
const MIN_STEP: Duration = Duration::from_millis(20);
/// One cycle of a placeholder's pulse, and how often it is redrawn.
const PULSE_PERIOD: f32 = 1.6;
const PULSE_STEP: Duration = Duration::from_millis(80);

/// When it started playing: the first time it was drawn.
struct Started(Instant);

/// An animated GIF.
pub struct Animation {
    frames: Frames,
    width: Length,
    height: Length,
    radius: f32,
    fit: ContentFit,
}

/// An animated GIF `w` by `h`, cropped to fill.
pub(super) fn animation<'a, M: 'a>(frames: Frames, w: f32, h: f32, radius: f32) -> Element<'a, M> {
    Element::new(Animation {
        frames,
        width: Length::Fixed(w),
        height: Length::Fixed(h),
        radius,
        fit: ContentFit::Cover,
    })
}

/// An animated GIF filling the space it is given, all of it in view, as the image
/// viewer shows it.
pub fn animation_fit<'a, M: 'a>(frames: Frames) -> Element<'a, M> {
    Element::new(Animation {
        frames,
        width: Length::Fill,
        height: Length::Fill,
        radius: 0.0,
        fit: ContentFit::Contain,
    })
}

impl<M> Widget<M, Theme, iced::Renderer> for Animation {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Started>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Started(Instant::now()))
    }

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
        tree: &mut Tree,
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
        {
            let start = tree.state.downcast_ref::<Started>().0;
            let (_, left) = self.frames.at(now.saturating_duration_since(start));
            shell.request_redraw_at(*now + left.max(MIN_STEP));
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let start = tree.state.downcast_ref::<Started>().0;
        let (frame, _) = self.frames.at(start.elapsed());
        image::draw(
            renderer,
            layout,
            frame,
            None,
            border::Radius::from(self.radius),
            self.fit,
            FilterMethod::default(),
            Rotation::default(),
            1.0,
            1.0,
        );
    }
}

/// A grey placeholder `w` by `h` with a gentle pulse while it waits.
pub(super) struct Pulse {
    size: Size,
    radius: f32,
}

pub(super) fn pulse<'a, M: 'a>(w: f32, h: f32, radius: f32) -> Element<'a, M> {
    Element::new(Pulse {
        size: Size::new(w, h),
        radius,
    })
}

/// Where every pulse is, from 0 to 1, from the wall clock so all placeholders on
/// screen breathe together.
fn pulse_now() -> f32 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let t = (secs % f64::from(PULSE_PERIOD)) as f32 / PULSE_PERIOD;
    0.5 - 0.5 * (t * std::f32::consts::TAU).cos()
}

impl<M> Widget<M, Theme, iced::Renderer> for Pulse {
    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fixed(self.size.width),
            Length::Fixed(self.size.height),
        )
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(self.size.width, self.size.height, self.size))
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
        {
            shell.request_redraw_at(*now + PULSE_STEP);
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let style = theme::loading(self.radius, pulse_now())(theme);
        if let Some(background) = style.background {
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: style.border,
                    ..renderer::Quad::default()
                },
                background,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_loop_and_say_when_they_change() {
        let h = image::Handle::from_rgba(1, 1, vec![0; 4]);
        let f = Frames(Arc::new(vec![
            (h.clone(), Duration::from_millis(100)),
            (h, Duration::from_millis(50)),
        ]));
        assert_eq!(f.at(Duration::from_millis(30)).1, Duration::from_millis(70));
        assert_eq!(
            f.at(Duration::from_millis(120)).1,
            Duration::from_millis(30)
        );
        assert_eq!(
            f.at(Duration::from_millis(160)).1,
            Duration::from_millis(90)
        );
    }

    #[test]
    fn pulses_stay_between_zero_and_one() {
        let p = pulse_now();
        assert!((0.0..=1.0).contains(&p));
    }
}
