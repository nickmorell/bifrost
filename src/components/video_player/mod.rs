//! Videos in posts: the poster with a play button, the inline player with its
//! controls, and the full-window view. The player shows frames it is given; playing
//! them is the `video` module's job.

mod moving;

pub use moving::Feed;

use super::avatar::{Picture, picture as still_picture};
use super::button::{ghost, icon_button};
use super::feedback::tip;
use super::icon::{Icon, icon};
use super::media::{MEDIA_W, badge};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, radius, size};
use iced::widget::{
    Row, Space, button, center, column, container, image, mouse_area, opaque, row, slider, stack,
    text,
};
use iced::{Alignment, ContentFit, Element, Length};
use std::sync::Arc;

/// What the player has to show.
pub struct Screen<'a> {
    /// The playing video's frames.
    pub feed: Arc<dyn Feed>,
    /// The video's still, shown until the first frame is ready.
    pub still: Option<&'a image::Handle>,
    pub failed: bool,
}

/// Where playback is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Playback {
    pub paused: bool,
    pub ended: bool,
    pub muted: bool,
    /// Seconds in, and the length in seconds.
    pub at: f64,
    pub len: f64,
    /// A live stream: no length to seek in, so a LIVE mark takes the seek bar's place.
    pub live: bool,
}

/// The messages the controls send.
#[derive(Debug, Clone)]
pub struct Remote<M> {
    pub toggle: M,
    pub mute: M,
    /// Sent when the seek bar is let go.
    pub seek: M,
    /// Make the view larger inline, or smaller in the large view.
    pub resize: M,
    /// Sent while the seek bar is dragged.
    pub scrub: fn(f32) -> M,
}

/// The size a video is shown at in a post.
pub fn inline_size(ratio: f32) -> (f32, f32) {
    let ratio = ratio.clamp(0.5, 3.0);
    // Tall videos are capped in height and narrowed to keep their shape.
    let h = (MEDIA_W / ratio).min(480.0);
    (h * ratio, h)
}

fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// The video's still `w` by `h` with something centred over it, and its ALT badge.
pub fn poster<'a, M: 'a>(
    still: Picture,
    w: f32,
    h: f32,
    middle: Element<'a, M>,
    alt: Option<&'a str>,
) -> Element<'a, M> {
    let mut layers = stack![
        still_picture(still, w, h, radius::M),
        container(middle).center_x(w).center_y(h),
    ];
    if let Some(alt) = alt {
        layers = layers.push(
            container(tip(badge("ALT"), alt))
                .padding(gap::S)
                .width(w)
                .height(h)
                .align_bottom(h)
                .align_left(w),
        );
    }
    layers.into()
}

/// The round play button over a poster.
pub fn play_button<'a, M: Clone + 'a>(play: M) -> Element<'a, M> {
    button(icon(Icon::Play, icon_size::HERO, Tone::OnMedia))
        .padding(gap::M + 2.0)
        .style(theme::video_button)
        .on_press(play)
        .into()
}

/// The picture, or the still until the first frame is ready.
fn picture<'a, M: 'a>(screen: &Screen<'a>, w: Length, h: Length) -> Element<'a, M> {
    let fit = |f: &image::Handle| {
        image(f.clone())
            .width(w)
            .height(h)
            .content_fit(ContentFit::Contain)
    };
    if screen.feed.frame().is_some() {
        return moving::moving(screen.feed.clone(), screen.still.cloned(), w, h);
    }
    let still: Element<'a, M> = match screen.still {
        Some(t) => fit(t).into(),
        None => Space::new().width(w).height(h).into(),
    };
    let label = if screen.failed {
        "Can't play this video"
    } else {
        "Loading…"
    };
    stack![still, center(badge(label)).width(w).height(h)].into()
}

/// The inline player, `w` by `h`: the picture with its controls along the bottom.
pub fn player<'a, M: Clone + 'a>(
    screen: Screen<'a>,
    playback: Playback,
    remote: Remote<M>,
    w: f32,
    h: f32,
) -> Element<'a, M> {
    let pic = mouse_area(picture(&screen, Length::Fixed(w), Length::Fixed(h)))
        .on_press(remote.toggle.clone())
        .interaction(iced::mouse::Interaction::Pointer);
    container(stack![
        pic,
        container(controls(playback, remote, false))
            .width(w)
            .height(h)
            .align_bottom(h)
    ])
    .width(w)
    .height(h)
    .clip(true)
    .style(theme::video_frame)
    .into()
}

fn control<'a, M: Clone + 'a>(i: Icon, label: &'a str, msg: M) -> Element<'a, M> {
    tip(
        button(icon(i, icon_size::L, Tone::OnMedia))
            .padding(gap::XS + 2.0)
            .style(theme::viewer_arrow)
            .on_press(msg),
        label,
    )
}

/// Play/pause, time, seek bar, sound and size.
fn controls<'a, M: Clone + 'a>(p: Playback, remote: Remote<M>, expanded: bool) -> Element<'a, M> {
    let (toggle, label) = if p.ended && !p.live {
        (Icon::Refresh, "Play again (Space)")
    } else if p.paused {
        (Icon::Play, "Play (Space)")
    } else {
        (Icon::Pause, "Pause (Space)")
    };
    let mut bar = Row::new().spacing(gap::S).align_y(Alignment::Center);
    bar = if p.live && p.ended {
        bar.push(
            text("The stream has ended")
                .size(size::CAPTION)
                .style(theme::on_media)
                .width(Length::Fill),
        )
    } else if p.live {
        bar.push(control(toggle, label, remote.toggle))
            .push(live_mark())
            .push(Space::new().width(Length::Fill))
    } else {
        bar.push(control(toggle, label, remote.toggle))
            .push(
                text(format!("{} / {}", clock(p.at), clock(p.len)))
                    .size(size::CAPTION)
                    .style(theme::on_media),
            )
            .push(
                slider(0.0..=p.len.max(0.1) as f32, p.at as f32, remote.scrub)
                    .step(0.1_f32)
                    .on_release(remote.seek)
                    .width(Length::Fill),
            )
    };
    bar = bar.push(if p.muted {
        control(Icon::VolumeOff, "Unmute", remote.mute)
    } else {
        control(Icon::Volume, "Mute", remote.mute)
    });
    bar = bar.push(if expanded {
        control(Icon::Close, "Close large view (Esc)", remote.resize)
    } else {
        control(Icon::Maximize, "Large view", remote.resize)
    });
    container(bar)
        .padding([gap::XS, gap::S])
        .width(Length::Fill)
        .style(theme::video_controls)
        .into()
}

/// The red LIVE mark on a live stream.
pub fn live_mark<'a, M: 'a>() -> Element<'a, M> {
    container(
        text("LIVE")
            .size(size::CAPTION)
            .font(theme::SEMIBOLD)
            .style(theme::on_media),
    )
    .padding([1.0, gap::S])
    .style(theme::live_badge)
    .into()
}

/// The video over the whole window, with a button labelled `web_label` to open it
/// on the web.
pub fn expanded<'a, M: Clone + 'a>(
    screen: Screen<'a>,
    playback: Playback,
    remote: Remote<M>,
    web_label: &'a str,
    open_web: M,
) -> Element<'a, M> {
    let top = row![
        Space::new().width(Length::Fill),
        ghost(web_label, open_web),
        tip(
            icon_button(Icon::Close, remote.resize.clone()),
            "Close (Esc)"
        ),
    ]
    .spacing(gap::S)
    .align_y(Alignment::Center);
    let pic =
        mouse_area(picture(&screen, Length::Fill, Length::Fill)).on_press(remote.toggle.clone());
    opaque(
        container(
            column![
                top,
                container(pic).height(Length::Fill),
                container(container(controls(playback, remote, true)).max_width(900))
                    .center_x(Length::Fill),
            ]
            .spacing(gap::M)
            .padding(gap::L),
        )
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
        Toggle,
        Mute,
        Seek,
        Resize,
        Scrub(f32),
        Web,
    }

    fn remote() -> Remote<Msg> {
        Remote {
            toggle: Msg::Toggle,
            mute: Msg::Mute,
            seek: Msg::Seek,
            resize: Msg::Resize,
            scrub: Msg::Scrub,
        }
    }

    /// A video with frames, or none yet.
    struct Fake(Option<image::Handle>);

    impl Feed for Fake {
        fn advance(&self) -> Option<std::time::Duration> {
            None
        }
        fn frame(&self) -> Option<(image::Handle, Option<image::Handle>)> {
            self.0.clone().map(|f| (f, None))
        }
    }

    fn nothing() -> Screen<'static> {
        Screen {
            feed: Arc::new(Fake(None)),
            still: None,
            failed: false,
        }
    }

    const PLAYING: Playback = Playback {
        paused: false,
        ended: false,
        muted: true,
        at: 75.9,
        len: 120.0,
        live: false,
    };

    #[test]
    fn sizes_and_times() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(75.9), "1:15");
        let (w, h) = inline_size(16.0 / 9.0);
        assert!((w - MEDIA_W).abs() < 0.5 && h < 480.0);
        // Portrait videos are capped in height.
        let (w, h) = inline_size(9.0 / 16.0);
        assert!((h - 480.0).abs() < f32::EPSILON);
        assert!(w < 300.0);
    }

    #[test]
    fn shows_time_and_loading() {
        let mut ui = simulator(player(nothing(), PLAYING, remote(), 540.0, 300.0));
        assert!(ui.find("1:15 / 2:00").is_ok());
        assert!(ui.find("Loading…").is_ok());
        let failed = Screen {
            failed: true,
            ..nothing()
        };
        let mut ui = simulator(player(failed, PLAYING, remote(), 540.0, 300.0));
        assert!(ui.find("Can't play this video").is_ok());
    }

    #[test]
    fn clicking_the_picture_toggles_and_the_web_button_opens() {
        let mut ui = simulator(player(nothing(), PLAYING, remote(), 540.0, 300.0));
        ui.point_at(iced::Point::new(270.0, 100.0));
        let _ = ui.simulate(simulator::click());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Msg::Toggle]);

        let mut ui = simulator(expanded(
            nothing(),
            PLAYING,
            remote(),
            "Open on bsky.app",
            Msg::Web,
        ));
        assert!(ui.click("Open on bsky.app").is_ok());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Msg::Web]);
    }

    #[test]
    fn a_playing_video_draws_its_frame() {
        let frame = image::Handle::from_rgba(2, 2, vec![255; 16]);
        let screen = Screen {
            feed: Arc::new(Fake(Some(frame))),
            ..nothing()
        };
        let mut ui = simulator(player(screen, PLAYING, remote(), 540.0, 300.0));
        assert!(ui.find("Loading…").is_err());
        assert!(ui.snapshot(&iced::Theme::Dark).is_ok());
    }

    #[test]
    fn the_poster_plays() {
        let poster = poster(
            Picture::Waiting { pulse: false },
            540.0,
            300.0,
            play_button(Msg::Toggle),
            Some("A heron"),
        );
        let mut ui = simulator(poster);
        assert!(ui.find("ALT").is_ok());
        ui.point_at(iced::Point::new(270.0, 150.0));
        let _ = ui.simulate(simulator::click());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Msg::Toggle]);
    }
}
