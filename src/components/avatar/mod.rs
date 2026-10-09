//! Pictures fetched from the network: profile pictures, feed pictures and other
//! remote images, with placeholders while they load or when there are none.

mod moving;

pub use moving::{Frames, animation_fit};

use super::icon::{Icon, icon};
use crate::theme::{self, Tone};
use iced::widget::{Space, container, image, stack, text};
use iced::{Alignment, ContentFit, Element};

/// A remote picture as far as it has got.
#[derive(Debug, Clone)]
pub enum Picture {
    Loaded(image::Handle),
    /// An animated GIF, playing from when it is first drawn.
    Animated(Frames),
    /// Still on its way. `pulse` animates the placeholder; it stays still for
    /// pictures that failed or when motion is reduced.
    Waiting {
        pulse: bool,
    },
}

/// A picture `w` by `h`, cropped to fill, or a grey placeholder while it loads.
pub fn picture<'a, M: 'a>(picture: Picture, w: f32, h: f32, radius: f32) -> Element<'a, M> {
    match picture {
        Picture::Loaded(handle) => image(handle)
            .width(w)
            .height(h)
            .content_fit(ContentFit::Cover)
            .border_radius(radius)
            .into(),
        Picture::Animated(frames) => moving::animation(frames, w, h, radius),
        Picture::Waiting { pulse: true } => moving::pulse(w, h, radius),
        Picture::Waiting { pulse: false } => container(Space::new().width(w).height(h))
            .style(theme::loading(radius, 0.0))
            .into(),
    }
}

/// A round profile picture, or a person silhouette for accounts without one.
pub fn avatar<'a, M: 'a>(pic: Option<Picture>, size: f32) -> Element<'a, M> {
    match pic {
        Some(p) => picture(p, size, size, size / 2.0),
        None => container(icon(Icon::User, (size * 0.55).round(), Tone::Faint))
            .width(size)
            .height(size)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(theme::placeholder(size / 2.0))
            .into(),
    }
}

/// The profile picture of someone streaming now: inside a red ring, with a LIVE mark
/// along its bottom edge when there is room for one. It takes the same space as
/// [`avatar`].
pub fn live_avatar<'a, M: 'a>(pic: Option<Picture>, size: f32) -> Element<'a, M> {
    let ring = container(avatar(pic, size - 8.0))
        .width(size)
        .height(size)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(theme::live_ring);
    if size < 40.0 {
        return ring.into();
    }
    let mark = container(
        text("LIVE")
            .size(theme::size::CAPTION - 2.0)
            .font(theme::SEMIBOLD)
            .style(theme::on_media),
    )
    .padding([0.0, theme::gap::XS])
    .style(theme::live_badge);
    stack![
        ring,
        container(mark)
            .width(size)
            .height(size)
            .align_x(Alignment::Center)
            .align_y(Alignment::End),
    ]
    .into()
}

/// A feed or list picture, or a # mark for ones without one.
pub fn feed_avatar<'a, M: 'a>(pic: Option<Picture>, size: f32) -> Element<'a, M> {
    match pic {
        Some(p) => picture(p, size, size, theme::radius::M),
        None => mark(Icon::Hash, size, Tone::Faint),
    }
}

/// An icon centred on a rounded placeholder square.
pub fn mark<'a, M: 'a>(i: Icon, size: f32, tint: Tone) -> Element<'a, M> {
    container(icon(i, (size * 0.5).round(), tint))
        .width(size)
        .height(size)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(theme::placeholder(theme::radius::M))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[test]
    fn every_state_draws() {
        let frame = image::Handle::from_rgba(2, 2, vec![255; 16]);
        let gif = Frames(std::sync::Arc::new(vec![
            (frame.clone(), std::time::Duration::from_millis(100)),
            (frame, std::time::Duration::from_millis(100)),
        ]));
        let views: [Element<'_, ()>; 9] = [
            avatar(None, 40.0),
            live_avatar(None, 48.0),
            live_avatar(None, 24.0),
            avatar(Some(Picture::Waiting { pulse: true }), 40.0),
            avatar(Some(Picture::Waiting { pulse: false }), 40.0),
            picture(Picture::Animated(gif.clone()), 80.0, 60.0, 8.0),
            animation_fit(gif),
            feed_avatar(None, 40.0),
            mark(Icon::List, 40.0, Tone::Accent),
        ];
        for view in views {
            assert!(simulator(view).snapshot(&iced::Theme::Dark).is_ok());
        }
    }
}
