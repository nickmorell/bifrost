//! What the app listens to: the window, files that failed to save, and timers for checking what is new and
//! playing video. GIFs and loading placeholders animate on their own.

use super::{App, Message, Phase, Route, chat, music, nav, pictures, video, window};
use crate::{storage, theme};
use iced::Subscription;
use iced::futures::{Stream, stream};
use std::time::Duration;

impl App {
    pub(super) fn subscription(&self) -> Subscription<Message> {
        let system_theme =
            theme::system::changes().map(|a| Message::Window(window::Msg::SystemTheme(a)));
        let tray = self.tray_subscription();
        let saves = Subscription::run(save_failures);
        let Phase::Ready = self.phase else {
            return Subscription::batch([system_theme, tray, saves]);
        };
        let background = !self.win.focused;
        let hidden = self.win.minimized;
        let mut subs = vec![
            system_theme,
            tray,
            saves,
            iced::keyboard::listen().map(|a| Message::Window(window::Msg::Key(a))),
            // The mouse's back button goes back, as in a browser.
            iced::event::listen_with(|event, _, _| match event {
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                    iced::mouse::Button::Back,
                )) => Some(Message::Nav(nav::Msg::Back)),
                iced::Event::Window(iced::window::Event::FileDropped(path)) => {
                    Some(Message::Window(window::Msg::Dropped(path)))
                }
                iced::Event::Window(iced::window::Event::Focused) => {
                    Some(Message::Window(window::Msg::Focus(true)))
                }
                iced::Event::Window(iced::window::Event::Unfocused) => {
                    Some(Message::Window(window::Msg::Focus(false)))
                }
                iced::Event::Window(iced::window::Event::Resized(size)) => {
                    Some(Message::Window(window::Msg::Resized(size)))
                }
                _ => None,
            }),
        ];
        // Little is polled while minimized: everything catches up when it comes back.
        // The unread count still is, slowly, for the tray badge and OS notifications.
        let unread = if background || hidden {
            BACKGROUND_POLL
        } else {
            UNREAD_POLL
        };
        subs.push(iced::time::every(unread).map(|_| Message::UnreadTick));
        if self.has_scheduled() {
            subs.push(
                iced::time::every(SCHEDULED_POLL)
                    .map(|_| Message::Drafts(super::drafts::Msg::Due(chrono::Utc::now()))),
            );
        }
        if !background
            && !hidden
            && self.current_timeline_key().is_some()
            && self.composer.is_none()
        {
            subs.push(iced::time::every(NEW_POSTS_POLL).map(|_| Message::Nav(nav::Msg::CheckNew)));
        }
        let in_chats = matches!(self.route(), Route::Chats | Route::Chat(_));
        // In the background the alerts check already fetches your chats.
        let chats_checked = background && self.alerts_fetch().1;
        if !self.chats.no_access && !hidden && !chats_checked {
            let every = if background {
                BACKGROUND_POLL
            } else if in_chats {
                chat::LIST_POLL
            } else {
                chat::LIST_POLL * 2
            };
            subs.push(iced::time::every(every).map(|_| Message::Chat(chat::Msg::Tick)));
        }
        if matches!(self.route(), Route::Chat(_)) && !hidden {
            let quiet = self
                .chats
                .active_at
                .is_none_or(|t| t.elapsed() > Duration::from_mins(1));
            let every = if background {
                chat::QUIET_POLL * 2
            } else if quiet {
                chat::QUIET_POLL
            } else {
                chat::OPEN_POLL
            };
            subs.push(iced::time::every(every).map(|_| Message::Chat(chat::Msg::Poll)));
        }
        if !hidden && let Some(chat) = self.live_subscription() {
            subs.push(chat);
        }
        if let Some(every) = self.video_tick() {
            subs.push(iced::time::every(every).map(|_| Message::Video(video::Msg::Tick)));
        }
        if self.music_ticking() {
            subs.push(iced::time::every(music::TICK).map(|_| Message::Music(music::Msg::Tick)));
        }
        if self.images.needs_tick() {
            subs.push(
                iced::time::every(Duration::from_secs(2))
                    .map(|_| Message::Pictures(pictures::Msg::Tick)),
            );
        }
        Subscription::batch(subs)
    }
}

/// Files written in the background that could not be saved.
fn save_failures() -> impl Stream<Item = Message> {
    stream::unfold(storage::failures(), |mut heard| async move {
        loop {
            heard.changed().await.ok()?;
            let failed = heard.borrow_and_update().clone();
            if let Some(failed) = failed {
                return Some((Message::SaveFailed(failed), heard));
            }
        }
    })
}

/// How often the open timeline is checked for newer posts.
const NEW_POSTS_POLL: Duration = Duration::from_mins(1);

/// How often the unread counts are checked while BiFrost is in the background.
const BACKGROUND_POLL: Duration = Duration::from_mins(5);

/// How often the unread notification count is checked.
const UNREAD_POLL: Duration = Duration::from_secs(30);

/// How often scheduled drafts are checked, while any is waiting.
const SCHEDULED_POLL: Duration = Duration::from_secs(30);
