//! What the window reports: keys, focus, size, the system theme, zoom and dropped
//! files, and moving the keyboard selection.

use super::composer::Composer;
use super::{App, Message, chat, composer, locate, nav};
use crate::theme::gap;
use iced::Task;
use iced::widget::scrollable;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum Msg {
    Key(iced::keyboard::Event),
    /// Move the keyboard selection this many posts (j / k).
    Move(isize),
    Moved(isize, locate::Layout),
    /// The window gained (`true`) or lost focus.
    Focus(bool),
    /// The window changed size. Windows reports a minimized window as zero-sized.
    Resized(iced::Size),
    /// The system switched between light and dark.
    SystemTheme(iced::theme::Mode),
    Zoom(f32),
    /// A file was dropped on the window: images go into the composer.
    Dropped(std::path::PathBuf),
}

/// The window as BiFrost last heard about it.
#[derive(Debug)]
pub(super) struct WindowState {
    /// Whether the window has focus, and since when it hasn't; polling slows down
    /// while BiFrost is in the background and stops while it is minimized.
    pub(super) focused: bool,
    pub(super) background_since: Option<Instant>,
    pub(super) minimized: bool,
    /// The window's size before zooming, for deciding how much of a timeline to draw
    /// and how the sidebar is laid out. None until it is first resized: it opens at
    /// `WINDOW` in layout units.
    pub(super) size: Option<iced::Size>,
    /// The OS light or dark setting, for the "Match system" appearance.
    pub(super) system_mode: iced::theme::Mode,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            focused: true,
            background_since: None,
            minimized: false,
            size: None,
            system_mode: iced::theme::Mode::None,
        }
    }
}

impl App {
    pub(super) fn window_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Key(event) => self.on_key(event),
            Msg::Move(step) => {
                let count = self
                    .current_timeline_key()
                    .and_then(|k| self.timelines.get(&k))
                    .map_or(0, |t| t.items().len());
                if count == 0 {
                    return Task::none();
                }
                locate::posts(self.scroll_key(), count)
                    .map(move |l| Message::Window(Msg::Moved(step, l)))
            }
            Msg::Moved(step, layout) => {
                let Some(i) = locate::next(&layout, self.selected, step) else {
                    return Task::none();
                };
                self.selected = Some(i);
                match layout.tops.get(i).copied().flatten() {
                    Some(top) => iced::widget::operation::scroll_to(
                        self.scroll_key(),
                        scrollable::AbsoluteOffset {
                            x: None,
                            y: Some((top - gap::M).max(0.0)),
                        },
                    ),
                    None => Task::none(),
                }
            }
            Msg::Dropped(path) => {
                if !crate::media::image::is_image_path(&path)
                    && !crate::media::files::is_video_path(&path)
                {
                    return Task::none();
                }
                if self.composer.is_none() {
                    self.composer = Some(self.new_composer(Composer::default()));
                }
                self.handle(Message::Composer(composer::Msg::Dropped(vec![path])))
            }
            Msg::Zoom(zoom) => {
                self.settings.zoom = zoom;
                self.save_settings();
                Task::none()
            }
            Msg::SystemTheme(mode) => {
                self.win.system_mode = mode;
                Task::none()
            }
            Msg::Focus(focused) => {
                self.win.focused = focused;
                let seen = self.alerts_seen();
                if !focused {
                    self.win.background_since.get_or_insert_with(Instant::now);
                    self.save_places(false);
                    return seen;
                }
                Task::batch([seen, self.catch_up()])
            }
            Msg::Resized(size) => {
                let minimized = size.width < 1.0 || size.height < 1.0;
                if !minimized {
                    // Reported in zoomed units; kept unzoomed so a zoom change applies.
                    self.win.size = Some(size * self.zoom());
                }
                let was = std::mem::replace(&mut self.win.minimized, minimized);
                if was && !minimized {
                    self.catch_up()
                } else {
                    Task::none()
                }
            }
        }
    }

    /// Puts the posts a background check found on top of their timeline. False when
    /// there were none.
    /// Back in front after a while away: check now rather than wait for the timers.
    pub(super) fn catch_up(&mut self) -> Task<Message> {
        let away = self.win.background_since.take();
        if self.win.minimized || away.is_none_or(|t| t.elapsed() <= Duration::from_secs(30)) {
            return Task::none();
        }
        Task::batch([
            self.handle(Message::UnreadTick),
            self.handle(Message::Nav(nav::Msg::CheckNew)),
            self.handle(Message::Chat(chat::Msg::Tick)),
        ])
    }
}
