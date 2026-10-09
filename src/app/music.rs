//! Playing Bandcamp releases in place: reading the tracks off the release page,
//! playing one after another, and the track list under the card.

use super::{App, Message};
use crate::api::{self, ApiError};
use crate::components::{self as ui, Icon, faint, icon_button};
use crate::integrations::bandcamp::{self, Release};
use crate::theme::{self, gap, size};
use crate::video::music::Music;
use iced::widget::{Column, row, text};
use iced::{Alignment, Element, Length, Task};
use std::time::Duration;

/// How often the track list's clock moves on.
pub(super) const TICK: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub enum Msg {
    /// Play the release on this page.
    Play(String),
    Page(String, Result<Vec<u8>, ApiError>),
    /// Play this track of the release.
    Track(usize),
    Song(usize, Result<Vec<u8>, ApiError>),
    Pause(bool),
    Stop,
    Tick,
}

/// The release that is playing, or loading.
pub(super) struct Listening {
    page: String,
    release: Option<Release>,
    track: usize,
    player: Option<Music>,
    error: Option<String>,
}

impl App {
    pub(super) fn music_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Play(page) => {
                self.music = Some(Listening {
                    page: page.clone(),
                    release: None,
                    track: 0,
                    player: None,
                    error: None,
                });
                Task::perform(
                    async move {
                        let r = api::fetch_bytes(&page).await;
                        (page, r)
                    },
                    |(p, r)| Message::Music(Msg::Page(p, r)),
                )
            }
            Msg::Page(page, result) => {
                let Some(l) = self.music.as_mut().filter(|l| l.page == page) else {
                    return Task::none();
                };
                let release = match result {
                    Ok(page) => bandcamp::release(&String::from_utf8_lossy(&page))
                        .ok_or_else(|| "Couldn't find the tracks on the release page.".to_string()),
                    Err(e) => Err(format!("Couldn't load the release page. {}", e.friendly())),
                };
                let first = release
                    .as_ref()
                    .ok()
                    .and_then(|r| r.tracks.iter().position(|t| t.stream.is_some()));
                match (release, first) {
                    (Ok(r), Some(i)) => {
                        l.release = Some(r);
                        self.music_update(Msg::Track(i))
                    }
                    (Ok(_), None) => {
                        l.error = Some(
                            "The artist hasn't made these tracks free to stream. Open it on Bandcamp to listen."
                                .into(),
                        );
                        Task::none()
                    }
                    (Err(e), _) => {
                        l.error = Some(e);
                        Task::none()
                    }
                }
            }
            Msg::Track(i) => {
                let Some(l) = self.music.as_mut() else {
                    return Task::none();
                };
                let Some(url) = l
                    .release
                    .as_ref()
                    .and_then(|r| r.tracks.get(i))
                    .and_then(|t| t.stream.clone())
                else {
                    return Task::none();
                };
                l.track = i;
                l.player = None;
                l.error = None;
                Task::perform(async move { api::fetch_bytes(&url).await }, move |r| {
                    Message::Music(Msg::Song(i, r))
                })
            }
            Msg::Song(i, result) => {
                let Some(l) = self.music.as_mut().filter(|l| l.track == i) else {
                    return Task::none();
                };
                match result
                    .map_err(|e| e.friendly())
                    .and_then(|mp3| Music::play(mp3).map_err(|e| e.to_string()))
                {
                    Ok(player) => l.player = Some(player),
                    Err(e) => l.error = Some(e),
                }
                Task::none()
            }
            Msg::Pause(paused) => {
                if let Some(p) = self.music.as_ref().and_then(|l| l.player.as_ref()) {
                    p.set_paused(paused);
                }
                Task::none()
            }
            Msg::Stop => {
                self.music = None;
                Task::none()
            }
            Msg::Tick => {
                // At the end of a track, the next one that can be played.
                let Some(l) = &self.music else {
                    return Task::none();
                };
                if !l.player.as_ref().is_some_and(Music::ended) {
                    return Task::none();
                }
                let next = l.release.as_ref().and_then(|r| {
                    (l.track + 1..r.tracks.len()).find(|&i| r.tracks[i].stream.is_some())
                });
                if let Some(i) = next {
                    self.music_update(Msg::Track(i))
                } else {
                    self.music = None;
                    Task::none()
                }
            }
        }
    }

    /// Whether the track list needs its clock moved on.
    pub(super) fn music_ticking(&self) -> bool {
        self.music.as_ref().is_some_and(|l| l.player.is_some())
    }

    /// Under a Bandcamp card: a Play button, or the track list while it plays.
    pub(super) fn music_view<'a>(&'a self, page: &str) -> Option<Element<'a, Message>> {
        if !bandcamp::is_release(page) {
            return None;
        }
        let Some(l) = self.music.as_ref().filter(|l| l.page == page) else {
            return Some(ui::icon_action(
                Icon::Play,
                "Play here",
                Message::Music(Msg::Play(page.to_string())),
            ));
        };
        let mut col = Column::new().spacing(gap::XS);
        let Some(release) = &l.release else {
            return Some(faint(l.error.as_deref().unwrap_or("Loading the tracks…")).into());
        };
        let paused = l.player.as_ref().is_some_and(Music::paused);
        let mut controls = row![
            text(format!("{} · {}", release.title, release.artist))
                .font(theme::MEDIUM)
                .size(size::SMALL)
                .width(Length::Fill)
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center);
        if l.player.is_some() {
            controls = controls.push(ui::tip(
                icon_button(
                    if paused { Icon::Play } else { Icon::Pause },
                    Message::Music(Msg::Pause(!paused)),
                ),
                if paused { "Play" } else { "Pause" },
            ));
        }
        controls = controls.push(ui::tip(
            icon_button(Icon::Close, Message::Music(Msg::Stop)),
            "Stop",
        ));
        col = col.push(controls);
        if let Some(e) = &l.error {
            col = col.push(faint(e.as_str()));
        }
        for (i, t) in release.tracks.iter().enumerate() {
            let current = i == l.track;
            let length = if current && let Some(p) = &l.player {
                format!("{} / {}", clock(p.time()), clock(t.seconds))
            } else {
                clock(t.seconds)
            };
            let title = text(format!("{}. {}", i + 1, t.title)).size(size::SMALL);
            let title = if current {
                title.font(theme::SEMIBOLD)
            } else {
                title
            };
            let line = row![title.width(Length::Fill), faint(length).size(size::CAPTION)]
                .spacing(gap::S)
                .align_y(Alignment::Center);
            col = col.push(match &t.stream {
                Some(_) if !current => ui::hand(line)
                    .on_press(Message::Music(Msg::Track(i)))
                    .into(),
                _ => Element::from(line),
            });
        }
        Some(ui::outlined(col).width(Length::Fill).into())
    }
}

/// "3:05".
fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::clock;

    #[test]
    fn track_lengths_read_as_minutes_and_seconds() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(185.4), "3:05");
        assert_eq!(clock(-3.0), "0:00");
    }
}
