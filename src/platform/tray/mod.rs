//! The system tray: an icon showing how much is unread, with "Open BiFrost" and "Quit"
//! in its menu. Windows and macOS keep the icon on the app's own thread; Linux needs
//! GTK, so there it lives on a GTK thread of its own. Where no tray can be made (a Linux
//! desktop without GTK or a tray), [`system`] returns `None` and the app does without.

pub(in crate::platform) mod icon;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// Something done with the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    /// The icon was clicked or "Open BiFrost" was chosen.
    Open,
    /// "Quit" was chosen.
    Quit,
}

pub type Events = UnboundedReceiver<TrayEvent>;

pub trait Tray {
    /// Shows how many things are unread; zero clears the badge.
    fn set_unread(&mut self, count: u64);
    /// What happens to the icon. Handed out once.
    fn events(&mut self) -> Option<Events>;
}

/// The system tray icon, or `None` where this system has no tray. On macOS and Windows
/// call it from the app's own thread once the app is running.
pub fn system() -> Option<Box<dyn Tray>> {
    let (tx, rx) = unbounded_channel();
    imp::start(tx).map(|tray| {
        Box::new(Native {
            tray,
            events: Some(rx),
        }) as Box<dyn Tray>
    })
}

struct Native {
    tray: imp::Handle,
    events: Option<Events>,
}

impl Tray for Native {
    fn set_unread(&mut self, count: u64) {
        self.tray.set_unread(count);
    }

    fn events(&mut self) -> Option<Events> {
        self.events.take()
    }
}

/// Builds the icon and its menu, and sends what happens to them to `events`.
fn build(events: UnboundedSender<TrayEvent>) -> Option<TrayIcon> {
    let open = MenuItem::new("Open BiFrost", true, None);
    let quit = MenuItem::new("Quit", true, None);
    let menu = Menu::with_items(&[&open, &quit]).ok()?;
    let (open_id, quit_id) = (open.id().clone(), quit.id().clone());
    let menu_events = events.clone();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let event = if e.id == open_id {
            TrayEvent::Open
        } else if e.id == quit_id {
            TrayEvent::Quit
        } else {
            return;
        };
        // Nobody listening means the window is closing.
        let _ = menu_events.send(event);
    }));
    TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = e
        {
            // Nobody listening means the window is closing.
            let _ = events.send(TrayEvent::Open);
        }
    }));
    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip(icon::tooltip(0))
        .with_icon(picture(0)?)
        .build()
        .ok()
}

fn picture(unread: u64) -> Option<Icon> {
    let side = icon::SIZE as u32;
    Icon::from_rgba(icon::pixels(unread), side, side).ok()
}

fn show_unread(tray: &TrayIcon, count: u64) {
    // The badge is a hint; the count is also in the window.
    let _ = tray.set_icon(picture(count));
    let _ = tray.set_tooltip(Some(icon::tooltip(count)));
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use super::{TrayEvent, TrayIcon, UnboundedSender, build, show_unread};

    /// The icon itself, owned by the app's thread.
    pub(super) struct Handle(TrayIcon);

    pub(super) fn start(events: UnboundedSender<TrayEvent>) -> Option<Handle> {
        build(events).map(Handle)
    }

    impl Handle {
        pub(super) fn set_unread(&self, count: u64) {
            show_unread(&self.0, count);
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{TrayEvent, UnboundedSender, build, show_unread};
    use gtk::glib::{self, ControlFlow};
    use std::sync::mpsc;
    use std::time::Duration;

    /// How often the GTK thread picks up new counts.
    const CHECK: Duration = Duration::from_millis(250);

    /// Sends new unread counts to the GTK thread that owns the icon.
    pub(super) struct Handle(mpsc::Sender<u64>);

    pub(super) fn start(events: UnboundedSender<TrayEvent>) -> Option<Handle> {
        let (counts, rx) = mpsc::channel::<u64>();
        let (ready, started) = mpsc::channel();
        std::thread::Builder::new()
            .name("tray".into())
            .spawn(move || {
                // `start` waits on `ready`, so these sends always land.
                if gtk::init().is_err() {
                    let _ = ready.send(false);
                    return;
                }
                let Some(tray) = build(events) else {
                    let _ = ready.send(false);
                    return;
                };
                let _ = ready.send(true);
                glib::timeout_add_local(CHECK, move || {
                    loop {
                        match rx.try_recv() {
                            Ok(count) => show_unread(&tray, count),
                            Err(mpsc::TryRecvError::Empty) => return ControlFlow::Continue,
                            // The app has gone: stop GTK so the thread ends.
                            Err(mpsc::TryRecvError::Disconnected) => {
                                gtk::main_quit();
                                return ControlFlow::Break;
                            }
                        }
                    }
                });
                gtk::main();
            })
            .ok()?;
        started
            .recv_timeout(Duration::from_secs(5))
            .ok()
            .filter(|ok| *ok)
            .map(|_| Handle(counts))
    }

    impl Handle {
        pub(super) fn set_unread(&self, count: u64) {
            // The tray's thread only stops when BiFrost quits.
            let _ = self.0.send(count);
        }
    }
}
