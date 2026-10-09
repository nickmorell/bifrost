//! The system tray: the icon's unread badge, its menu, and closing the window into it.
//! Where there is no tray, closing the window quits as usual.

use super::{App, Message};
use crate::platform::tray::{self, Events, TrayEvent};
use iced::futures::{Stream, stream};
use iced::window::{self, Mode};
use iced::{Subscription, Task};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum Msg {
    /// The tray icon was clicked, or something in its menu chosen.
    Event(TrayEvent),
    /// The window's close button was pressed.
    Close(window::Id),
}

/// The tray's events, waiting for the subscription that listens to them.
#[derive(Clone, Default)]
pub(super) struct Listener(Arc<Mutex<Option<Events>>>);

/// There is only ever one tray, so every `Listener` is the same subscription.
impl Hash for Listener {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "tray".hash(state);
    }
}

/// The tray icon and what it shows.
#[derive(Default)]
pub(super) struct TrayState {
    /// The icon, once made, where this system has a tray.
    pub(super) icon: Option<Box<dyn tray::Tray>>,
    pub(super) tried: bool,
    pub(super) events: Listener,
    /// The unread count the icon shows.
    pub(super) unread: u64,
    /// The window was closed into the tray.
    pub(super) hidden: bool,
}

impl App {
    pub(super) fn tray_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Close(id) if self.settings.close_to_tray && self.tray.icon.is_some() => {
                self.save_places(false);
                self.tray.hidden = true;
                self.win.focused = false;
                self.win
                    .background_since
                    .get_or_insert_with(std::time::Instant::now);
                window::set_mode(id, Mode::Hidden)
            }
            Msg::Close(_) | Msg::Event(TrayEvent::Quit) => {
                self.save_places(true);
                self.keep_loaded(true);
                iced::exit()
            }
            Msg::Event(TrayEvent::Open) => {
                self.tray.hidden = false;
                window::oldest().and_then(|id| {
                    Task::batch([window::set_mode(id, Mode::Windowed), window::gain_focus(id)])
                })
            }
        }
    }

    /// Puts the icon in the tray, the first time a session starts. It has to be made
    /// once the app is running, on its own thread, which is why it isn't made at boot.
    pub(super) fn start_tray(&mut self) {
        if self.tray.tried {
            return;
        }
        self.tray.tried = true;
        self.tray.icon = tray::system();
        if let Some(t) = &mut self.tray.icon
            && let Ok(mut events) = self.tray.events.0.lock()
        {
            *events = t.events();
        }
    }

    /// Shows unread notifications and messages on the icon, when the count changed.
    pub(super) fn sync_tray(&mut self) {
        let unread = self.notifications.unread() + self.chats.unread;
        if let Some(t) = &mut self.tray.icon
            && unread != self.tray.unread
        {
            self.tray.unread = unread;
            t.set_unread(unread);
        }
    }

    /// The window's close button, and the tray icon once there is one.
    pub(super) fn tray_subscription(&self) -> Subscription<Message> {
        let close = window::close_requests().map(|id| Message::Tray(Msg::Close(id)));
        if self.tray.icon.is_none() {
            return close;
        }
        Subscription::batch([
            close,
            Subscription::run_with(self.tray.events.clone(), listen),
        ])
    }
}

fn listen(listener: &Listener) -> impl Stream<Item = Message> + use<> {
    let events = listener.0.lock().ok().and_then(|mut e| e.take());
    stream::unfold(events, |events| async move {
        let mut events = events?;
        let event = events.recv().await?;
        Some((Message::Tray(Msg::Event(event)), Some(events)))
    })
}
