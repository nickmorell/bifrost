//! Pictures in posts: the image grid, animated GIFs, small thumbnails for quoted
//! posts, and the GIF and ALT badges over them. The full-screen viewer is in
//! `viewer.rs`.

mod viewer;

pub use viewer::{Shown, viewer};

use super::avatar::{Picture, picture};
use super::button::hand;
use super::feedback::tip;
use super::icon::{Icon, icon};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, radius, size};
use iced::widget::{Row, Space, column, container, row, stack, text};
use iced::{Element, Length};

/// Width of media in a post.
pub const MEDIA_W: f32 = 540.0;
const GAP: f32 = gap::XS;

/// One image of a post: its picture so far and its description.
pub struct Tile<'a> {
    pub picture: Picture,
    pub alt: &'a str,
}

/// A post's images: one large, two side by side, three as one tall plus two stacked,
/// four as a 2×2 grid. `ratio` is the first image's width over height, if known.
/// Pressing an image sends `open` with its index.
pub fn images<'a, M: Clone + 'a>(
    tiles: Vec<Tile<'a>>,
    ratio: Option<f32>,
    open: impl Fn(usize) -> M,
) -> Element<'a, M> {
    let n = tiles.len();
    let half = (MEDIA_W - GAP) / 2.0;
    let size_of = |i: usize| match n {
        1 => {
            let ratio = ratio.unwrap_or(1.5).clamp(0.5, 3.0);
            // Tall images are capped in height and narrowed to keep their shape.
            let h = (MEDIA_W / ratio).min(520.0);
            (h * ratio, h)
        }
        2 => (half, half * 0.9),
        3 if i == 0 => (half, half * 1.2),
        3 => (half, (half * 1.2 - GAP) / 2.0),
        _ => (half, half * 0.66),
    };
    let mut cells = tiles.into_iter().take(4).enumerate().map(|(i, t)| {
        let (w, h) = size_of(i);
        let pic = hand(picture(t.picture, w, h, radius::M)).on_press(open(i));
        with_badges(pic.into(), None, t.alt)
    });
    let mut next = || cells.next().unwrap_or_else(|| Space::new().into());
    match n {
        0 => Space::new().into(),
        1 => next(),
        2 => row![next(), next()].spacing(GAP).into(),
        3 => row![next(), column![next(), next()].spacing(GAP)]
            .spacing(GAP)
            .into(),
        _ => column![
            row![next(), next()].spacing(GAP),
            row![next(), next()].spacing(GAP),
        ]
        .spacing(GAP)
        .into(),
    }
}

/// An animated GIF link, shown about the height Bluesky shows them at. `ratio` is its
/// width over height, if the link says.
pub fn gif<'a, M: Clone + 'a>(
    pic: Picture,
    ratio: Option<f32>,
    alt: &'a str,
    open: M,
) -> Element<'a, M> {
    let ratio = ratio.unwrap_or(1.5).clamp(0.5, 3.0);
    let w = MEDIA_W.min(300.0 * ratio);
    let h = w / ratio;
    let img = hand(picture(pic, w, h, radius::M)).on_press(open);
    with_badges(img.into(), Some("GIF"), alt)
}

/// A quoted post's images, as a row of small squares.
pub fn thumbs<'a, M: Clone + 'a>(
    pictures: Vec<Picture>,
    open: impl Fn(usize) -> M,
) -> Element<'a, M> {
    let mut r = Row::new().spacing(GAP);
    for (i, pic) in pictures.into_iter().enumerate() {
        r = r.push(hand(picture(pic, 88.0, 88.0, radius::S)).on_press(open(i)));
    }
    r.into()
}

/// A quoted post's video: its still with a play mark.
pub fn video_thumb<'a, M: 'a>(pic: Picture) -> Element<'a, M> {
    stack![
        picture(pic, 160.0, 90.0, radius::S),
        container(icon(Icon::Play, icon_size::XL, Tone::OnMedia))
            .center_x(160)
            .center_y(90),
    ]
    .into()
}

/// Lays an image's GIF and ALT badges over its bottom-left corner.
fn with_badges<'a, M: 'a>(
    content: Element<'a, M>,
    kind: Option<&'a str>,
    alt: &'a str,
) -> Element<'a, M> {
    let mut badges = Row::new().spacing(GAP);
    if let Some(k) = kind {
        badges = badges.push(badge(k));
    }
    if !alt.is_empty() {
        badges = badges.push(tip(badge("ALT"), alt));
    }
    stack![
        content,
        container(badges)
            .padding(gap::S)
            .align_bottom(Length::Fill)
            .align_left(Length::Fill)
    ]
    .into()
}

/// A small label over a picture or video, such as "ALT" or "Loading…".
pub fn badge<'a, M: 'a>(label: &'a str) -> Element<'a, M> {
    container(text(label).size(size::CAPTION - 1.0).font(theme::SEMIBOLD))
        .padding([1.0, 5.0])
        .style(theme::media_badge)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;
    use iced_test::simulator;

    fn waiting(alt: &str) -> Tile<'_> {
        Tile {
            picture: Picture::Waiting { pulse: false },
            alt,
        }
    }

    /// The index sent by clicking at `at` in a grid of `n` images.
    fn open_at(n: usize, at: Point) -> Vec<usize> {
        let tiles = (0..n).map(|_| waiting("")).collect();
        let mut ui = simulator(images(tiles, None, |i| i));
        ui.point_at(at);
        let _ = ui.simulate(simulator::click());
        ui.into_messages().collect()
    }

    #[test]
    fn each_image_opens_at_its_own_index() {
        assert_eq!(open_at(1, Point::new(10.0, 10.0)), [0]);
        assert_eq!(open_at(2, Point::new(400.0, 10.0)), [1]);
        // Three: one tall on the left, two stacked on the right.
        assert_eq!(open_at(3, Point::new(400.0, 300.0)), [2]);
        assert_eq!(open_at(4, Point::new(10.0, 300.0)), [2]);
    }

    #[test]
    fn badges_and_thumbnails_draw() {
        let views: [Element<'_, ()>; 3] = [
            gif(Picture::Waiting { pulse: true }, Some(2.0), "a cat", ()),
            thumbs(vec![Picture::Waiting { pulse: false }; 3], |_| ()),
            video_thumb(Picture::Waiting { pulse: false }),
        ];
        for view in views {
            assert!(simulator(view).snapshot(&iced::Theme::Dark).is_ok());
        }
        let mut ui = simulator(images(vec![waiting("A dog")], Some(1.0), |_| ()));
        assert!(ui.find("ALT").is_ok());
    }
}
