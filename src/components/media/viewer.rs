//! The full-screen image viewer, drawn over everything else.

use super::super::avatar::{Picture, animation_fit};
use super::super::button::icon_button;
use super::super::feedback::tip;
use super::super::icon::{Icon, icon};
use super::super::text::faint;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{
    Column, Row, Space, button, center, container, image, opaque, row, scrollable, text,
};
use iced::{Alignment, Element, Length, Padding};

/// The image the viewer is showing.
pub struct Shown<'a> {
    /// The picture as far as it has loaded.
    pub picture: Picture,
    /// Which of `count` images this is, from 0.
    pub index: usize,
    pub count: usize,
    pub alt: &'a str,
}

/// The viewer: the image, zoomable, with arrows between images, its description and
/// Save and Close buttons. The arrows send `step` with -1 or 1.
pub fn viewer<'a, M: Clone + 'a>(
    shown: Shown<'a>,
    save: M,
    close: M,
    step: impl Fn(isize) -> M,
) -> Element<'a, M> {
    let many = shown.count > 1;

    let picture: Element<'a, M> = match shown.picture {
        Picture::Loaded(h) => image::viewer(h)
            .width(Length::Fill)
            .height(Length::Fill)
            .min_scale(1.0)
            .max_scale(8.0)
            .into(),
        Picture::Animated(frames) => animation_fit(frames),
        Picture::Waiting { .. } => center(faint("Loading…")).into(),
    };

    let mut top = Row::new().spacing(gap::S).align_y(Alignment::Center);
    if many {
        top = top.push(faint(format!("{} / {}", shown.index + 1, shown.count)));
    }
    top = top
        .push(Space::new().width(Length::Fill))
        .push(tip(icon_button(Icon::Download, save), "Save image"))
        .push(tip(icon_button(Icon::Close, close), "Close (Esc)"));

    let arrow = |i: Icon, by: isize, enabled: bool| -> Element<'a, M> {
        let b: Element<'a, M> = if enabled {
            button(icon(i, icon_size::XXL, Tone::OnMedia))
                .padding(gap::S)
                .style(theme::viewer_arrow)
                .on_press(step(by))
                .into()
        } else {
            Space::new().width(38).height(38).into()
        };
        container(b).center_y(Length::Fill).into()
    };
    let middle: Element<'a, M> = if many {
        row![
            arrow(Icon::ChevronLeft, -1, shown.index > 0),
            container(picture).width(Length::Fill).height(Length::Fill),
            arrow(Icon::ChevronRight, 1, shown.index + 1 < shown.count),
        ]
        .spacing(gap::M)
        .into()
    } else {
        picture
    };

    let mut body = Column::new()
        .spacing(gap::M)
        .padding(Padding::from(gap::L).left(gap::XL).right(gap::XL))
        .push(top)
        .push(container(middle).height(Length::Fill));
    if !shown.alt.is_empty() {
        body = body.push(
            container(
                scrollable(text(shown.alt).size(size::BODY).style(theme::on_media))
                    .height(Length::Shrink),
            )
            .max_height(140)
            .padding([gap::S, gap::M])
            .center_x(Length::Fill)
            .style(theme::overlay),
        );
    }
    opaque(
        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::viewer_backdrop),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Save,
        Close,
        Step(isize),
    }

    #[test]
    fn shows_position_and_description() {
        let shown = Shown {
            picture: Picture::Waiting { pulse: false },
            index: 1,
            count: 3,
            alt: "A heron",
        };
        let mut ui = simulator(viewer(shown, Msg::Save, Msg::Close, Msg::Step));
        assert!(ui.find("2 / 3").is_ok());
        assert!(ui.find("A heron").is_ok());
        assert!(ui.find("Loading…").is_ok());
    }

    #[test]
    fn a_single_image_has_no_count() {
        let shown = Shown {
            picture: Picture::Waiting { pulse: false },
            index: 0,
            count: 1,
            alt: "",
        };
        let mut ui = simulator(viewer(shown, Msg::Save, Msg::Close, Msg::Step));
        assert!(ui.find("1 / 1").is_err());
    }
}
