//! Videos in posts: which video is playing, its messages, and the adapters that
//! hand it to the `video_player` component.

use super::util::picture_of;
use super::{App, Message, nav};
use crate::components::{self as ui, Picture, ghost, video_player as vp};
use crate::theme::gap;
use crate::video::{Pictures, Player, Status};
use iced::widget::{column, row};
use iced::{Alignment, Element, Length, Task};
use std::sync::Arc;
use std::time::Duration;

/// How often the controls (time, seek bar) catch up while a video plays. The picture
/// moves on by itself at the video's own frame rate.
const CONTROLS_STEP: Duration = Duration::from_millis(250);

/// The video that is playing. Only one plays at a time.
pub struct Playing {
    /// The post it belongs to.
    pub post: String,
    pub player: Player,
    /// Its pictures, as the player component draws them.
    feed: Arc<dyn vp::Feed>,
    /// Shown over the whole window.
    pub expanded: bool,
    /// Where the seek bar is being dragged to, before it is let go.
    scrub: Option<f32>,
    /// Where it plays in a browser (the post on bsky.app, or the stream on
    /// stream.place), for when it can't be played here.
    web: String,
    playlist: String,
}

impl std::fmt::Debug for Playing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playing")
            .field("post", &self.post)
            .finish_non_exhaustive()
    }
}

/// A playing video's pictures, handed to the player component.
struct Feed(Pictures);

impl vp::Feed for Feed {
    fn advance(&self) -> Option<Duration> {
        self.0.advance()
    }

    fn due(&self) -> Option<iced::widget::image::Handle> {
        self.0.due()
    }

    fn ready(&self) -> Option<iced::widget::image::Handle> {
        self.0.ready()
    }

    fn uploaded(&self, frame: iced::advanced::image::Allocation) {
        self.0.uploaded(frame);
    }
}

impl Playing {
    /// Whether this is `playlist`, still playing or ready to.
    pub fn plays(&self, playlist: &str) -> bool {
        self.playlist == playlist
            && !matches!(self.player.status(), Status::Failed(_) | Status::Ended)
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Play {
        post: String,
        playlist: String,
        web: String,
    },
    Toggle,
    Mute,
    Scrub(f32),
    Seek,
    /// Jump by this many seconds.
    Skip(f32),
    Expand,
    Collapse,
    Close,
    Tick,
}

impl App {
    pub(super) fn video_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Play {
                post,
                playlist,
                web,
            } => {
                let player = Player::open(&playlist, self.settings.video_muted);
                self.video = Some(Playing {
                    post,
                    feed: Arc::new(Feed(player.pictures())),
                    player,
                    expanded: false,
                    scrub: None,
                    web,
                    playlist,
                });
            }
            Msg::Tick => {
                // Moves on even while the picture is out of sight, so it can end.
                if let Some(v) = &self.video {
                    v.player.pictures().advance();
                }
            }
            Msg::Toggle => {
                if let Some(v) = &self.video {
                    match v.player.status() {
                        Status::Ended => {
                            v.player.seek(0.0);
                            v.player.set_paused(false);
                        }
                        Status::Failed(_) => {}
                        _ => v.player.set_paused(!v.player.paused()),
                    }
                }
            }
            Msg::Mute => {
                if let Some(v) = &self.video {
                    let muted = !v.player.muted();
                    v.player.set_muted(muted);
                    self.settings.video_muted = muted;
                    self.save_settings();
                }
            }
            Msg::Scrub(at) => {
                if let Some(v) = &mut self.video {
                    v.scrub = Some(at);
                }
            }
            Msg::Seek => {
                if let Some(v) = &mut self.video
                    && let Some(at) = v.scrub.take()
                {
                    v.player.seek(f64::from(at));
                }
            }
            Msg::Skip(by) => {
                if let Some(v) = &self.video {
                    let (pos, _) = v.player.progress();
                    v.player.seek(pos + f64::from(by));
                }
            }
            Msg::Expand => {
                if let Some(v) = &mut self.video {
                    v.expanded = true;
                }
            }
            Msg::Collapse => {
                if let Some(v) = &mut self.video {
                    v.expanded = false;
                }
            }
            Msg::Close => self.video = None,
        }
        Task::none()
    }

    /// Whether the video needs picture updates.
    /// How often to redraw for the playing video, if it needs redrawing at all.
    pub(super) fn video_tick(&self) -> Option<Duration> {
        self.video
            .as_ref()
            .filter(|v| v.player.active())
            .map(|_| CONTROLS_STEP)
    }

    /// The playing video, if it is the one in `post`.
    fn playing(&self, post: &str) -> Option<&Playing> {
        self.video.as_ref().filter(|v| v.post == post)
    }
}

fn msg(m: Msg) -> Message {
    Message::Video(m)
}

/// The still for a video, as far as it has loaded.
fn still(app: &App, thumb: Option<&str>) -> Picture {
    thumb.map_or(Picture::Waiting { pulse: false }, |t| picture_of(app, t))
}

/// What the player should show for `p`.
fn screen<'a>(app: &'a App, p: &'a Playing, thumb: Option<&str>) -> vp::Screen<'a> {
    vp::Screen {
        feed: p.feed.clone(),
        still: thumb.and_then(|t| app.image(t)),
        failed: matches!(p.player.status(), Status::Failed(_)),
    }
}

fn playback(p: &Playing) -> vp::Playback {
    let (pos, len) = p.player.progress();
    vp::Playback {
        paused: p.player.paused(),
        ended: p.player.status() == Status::Ended,
        muted: p.player.muted(),
        at: p.scrub.map_or(pos, f64::from),
        len,
        live: p.player.live(),
    }
}

fn remote(expanded: bool) -> vp::Remote<Message> {
    vp::Remote {
        toggle: msg(Msg::Toggle),
        mute: msg(Msg::Mute),
        seek: msg(Msg::Seek),
        resize: msg(if expanded { Msg::Collapse } else { Msg::Expand }),
        scrub: |v| Message::Video(Msg::Scrub(v)),
    }
}

/// A post's video: the poster with a play button, or the player once started.
pub fn video_view<'a>(
    app: &'a App,
    post: &'a str,
    playlist: &'a str,
    thumb: Option<&'a str>,
    alt: Option<&'a str>,
    ratio: Option<f32>,
    web: String,
) -> Element<'a, Message> {
    let playing = app.playing(post);
    let ratio = ratio
        .or_else(|| {
            playing
                .and_then(|p| p.player.size())
                .map(|(w, h)| w as f32 / h.max(1) as f32)
        })
        .unwrap_or(16.0 / 9.0);
    let (w, h) = vp::inline_size(ratio);
    let alt = alt.filter(|a| !a.is_empty());

    let content: Element<'a, Message> = match playing {
        Some(p) if !p.expanded => {
            vp::player(screen(app, p, thumb), playback(p), remote(false), w, h)
        }
        Some(_) => vp::poster(
            still(app, thumb),
            w,
            h,
            ui::faint("Playing in the large view").into(),
            None,
        ),
        None => {
            let play = vp::play_button(msg(Msg::Play {
                post: post.to_string(),
                playlist: playlist.to_string(),
                web: web.clone(),
            }));
            vp::poster(still(app, thumb), w, h, play, alt)
        }
    };
    let mut col = column![content].spacing(gap::XS);
    if let Some(Status::Failed(why)) = playing.map(|p| p.player.status()) {
        col = col.push(
            row![
                ui::faint(why).width(Length::Fill),
                ghost("Play on bsky.app", Message::Nav(nav::Msg::OpenUrl(web))),
            ]
            .align_y(Alignment::Center),
        );
    }
    col.into()
}

/// A live or past stream `w` by `h`: the player when `key` is playing, otherwise its
/// still with a play button that sends `watch`.
pub fn stream_view<'a>(
    app: &'a App,
    key: &str,
    thumb: Option<&'a str>,
    watch: Message,
    w: f32,
    h: f32,
) -> Element<'a, Message> {
    let playing = app.playing(key);
    let content = match playing {
        Some(p) if !p.expanded => {
            vp::player(screen(app, p, thumb), playback(p), remote(false), w, h)
        }
        Some(_) => vp::poster(
            still(app, thumb),
            w,
            h,
            ui::faint("Playing in the large view").into(),
            None,
        ),
        None => vp::poster(still(app, thumb), w, h, vp::play_button(watch), None),
    };
    let mut col = column![content].spacing(gap::XS).width(w);
    if let Some(p) = playing
        && let Status::Failed(why) = p.player.status()
    {
        col = col.push(
            row![
                ui::faint(why).width(Length::Fill),
                ghost(
                    web_label(&p.web),
                    Message::Nav(nav::Msg::Browse(p.web.clone()))
                ),
            ]
            .align_y(Alignment::Center),
        );
    }
    col.into()
}

/// The button that opens a video where it came from.
fn web_label(web: &str) -> &'static str {
    if web.starts_with(crate::api::streamplace::NODE) {
        "Open on stream.place"
    } else {
        "Open on bsky.app"
    }
}

/// The video over the whole window.
pub fn expanded_view<'a>(app: &'a App, p: &'a Playing) -> Element<'a, Message> {
    vp::expanded(
        screen(app, p, None),
        playback(p),
        remote(true),
        web_label(&p.web),
        Message::Nav(nav::Msg::Browse(p.web.clone())),
    )
}
