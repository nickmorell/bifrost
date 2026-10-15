//! The iced desktop app.
//!
//! One [`App`] holds every cache (timelines, threads, profiles, images) so that an
//! action such as a like updates the post wherever it is shown. This file only builds
//! the app and routes each [`Message`] to the part of the app it belongs to; the
//! submodules do the work and draw the screens.

mod account;
mod actions;
mod alerts;
mod archive;
mod articles;
mod atstore;
mod cache;
mod chat;
mod collections;
mod compose;
mod composer;
mod deck;
mod drafts;
mod feeds;
mod filters;
mod images;
mod interaction;
mod keys;
mod lists;
mod live;
mod locate;
mod login;
mod media;
mod message;
mod moderation;
mod music;
mod nav;
mod notifications;
mod people;
mod pictures;
mod places;
mod popups;
mod post;
mod prefs;
mod privacy;
mod profile;
mod screens;
mod search;
mod settings;
mod shell;
mod stats;
mod subscription;
mod thread;
mod timeline;
mod translate;
mod tray;
mod updates;
mod util;
mod video;
mod viewer;
mod window;

use crate::api::{self, AuthorFilter, Client};
use crate::auth::atrium::AtriumBackend;
use crate::auth::store::IndexLocation;
use crate::auth::{AccountManager, AccountSummary};
use crate::integrations::{self, Integration};
use crate::model::{AtUri, Did, Preferences, Profile};
use crate::storage::{image_cache, secrets::KeyringStore, settings::Settings};
use crate::theme;
use composer::Composer;
use iced::Task;
use login::Login;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

pub fn run() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .font(theme::fonts::REGULAR)
        .font(theme::fonts::MEDIUM)
        .font(theme::fonts::SEMIBOLD)
        .font(theme::fonts::BOLD)
        .default_font(theme::FONT)
        .subscription(App::subscription)
        .scale_factor(App::zoom)
        // Closing may only hide the window in the tray; `tray` decides.
        .exit_on_close_request(false)
        .window(iced::window::Settings {
            size: WINDOW,
            min_size: Some(iced::Size::new(720.0, 560.0)),
            position: iced::window::Position::Centered,
            icon: window::icon(),
            platform_specific: window::platform_specific(),
            ..Default::default()
        })
        .run()
}

/// The window's size when it opens.
const WINDOW: iced::Size = iced::Size::new(1100.0, 820.0);

pub use crate::features::profile::Tab as ProfileTab;
pub use message::{LinkTarget, Menu, Message, Route, Toggle};
pub use timeline::{Key as TimelineKey, item_key};

enum Phase {
    Booting,
    Login(Login),
    Ready,
}

pub struct App {
    accounts: Arc<api::Accounts>,
    /// The signed-in accounts, re-read whenever they change rather than on every render.
    account_list: Vec<AccountSummary>,
    client: Client,
    settings: Settings,
    integrations: Vec<Box<dyn Integration>>,
    phase: Phase,

    my_did: Did,
    me: Option<Profile>,
    prefs: Preferences,
    feeds: feeds::Feeds,

    timelines: timeline::Timelines,
    threads: thread::Threads,
    profile: crate::features::profile::Profiles,
    people: people::People,
    /// Preference edits made here and not yet saved, oldest first.
    pref_saver: crate::features::prefs::Saver,
    moderation: moderation::Moderation,
    notifications: notifications::Notifications,
    settings_screen: crate::features::settings::Screen,
    search: search::Search,

    routes: Vec<Route>,
    /// How far each screen was scrolled, by scroll key, so Back returns to the same place.
    scroll_offsets: HashMap<String, f32>,
    images: images::ImageCache,
    viewer: Option<media::Viewer>,
    /// The window: focus, size, minimized, light or dark.
    win: window::WindowState,
    /// Chats are loaded after the first timeline, so they don't hold it up.
    chats_deferred: bool,
    revealed: HashSet<AtUri>,
    /// The post picked with j / k in the current timeline, by index.
    selected: Option<usize>,
    /// Menus and small dialogs that are open.
    popups: popups::Popups,
    /// Where you were reading in each feed.
    reading: places::Reading,
    actions: actions::Actions,
    /// Posts being deleted, kept to put back if the server refuses.
    deleting: actions::Deleting,
    composer: Option<Composer>,
    interaction_edit: Option<interaction::Who>,
    deck: deck::Deck,
    translations: translate::Translations,
    chats: chat::Chats,
    lists: lists::Lists,
    live: live::Live,
    atstore: atstore::Store,
    /// The video that is playing, if any.
    video: Option<video::Playing>,
    drafts: drafts::Drafts,
    articles: articles::Articles,
    /// The Bandcamp release playing, if any.
    music: Option<music::Listening>,
    /// Posts kept on this computer for searching and reading offline.
    archive: crate::storage::archive::Archive,
    /// Bluesky couldn't be reached the last time it was asked.
    offline: bool,
    collections: collections::Collections,
    privacy: privacy::Privacy,
    stats: stats::Mine,
    /// Stops a browser sign-in that is waiting for the person.
    sign_in_abort: Option<iced::task::Handle>,
    toast: Option<String>,
    /// Bumped whenever a new toast appears, so only the latest one is auto-dismissed.
    toast_seq: u64,
    /// Bumped whenever the signed-in session changes; see [`Message::Session`].
    session: u64,
    alerts: crate::features::alerts::Alerts,
    notifier: Arc<dyn crate::platform::notify::Notifier>,
    tray: tray::TrayState,
    /// What the opt-in update check found this run.
    update: crate::features::settings::UpdateCheck,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let index = IndexLocation::default_file().unwrap_or_else(|_| IndexLocation::memory());
        let accounts = Arc::new(AccountManager::new(
            AtriumBackend::new(),
            KeyringStore::default(),
            index,
        ));
        let mut app = App {
            client: Client::new(accounts.clone()),
            account_list: accounts.accounts().unwrap_or_default(),
            accounts,
            settings: Settings::load(),
            integrations: integrations::all(),
            phase: Phase::Booting,
            my_did: Did::default(),
            me: None,
            prefs: Preferences::default(),
            feeds: feeds::Feeds::default(),
            timelines: timeline::Timelines::default(),
            threads: thread::Threads::default(),
            profile: crate::features::profile::Profiles::default(),
            people: people::People::default(),
            pref_saver: crate::features::prefs::Saver::default(),
            moderation: moderation::Moderation::default(),
            notifications: notifications::Notifications::default(),
            settings_screen: crate::features::settings::Screen::default(),
            search: search::Search::default(),
            routes: vec![Route::Home],
            scroll_offsets: HashMap::new(),
            images: images::ImageCache::default(),
            viewer: None,
            win: window::WindowState::default(),
            chats_deferred: false,
            revealed: HashSet::new(),
            selected: None,
            popups: popups::Popups::default(),
            reading: places::Reading::default(),
            actions: actions::Actions::default(),
            deleting: actions::Deleting::new(),
            composer: None,
            interaction_edit: None,
            deck: deck::Deck::default(),
            translations: translate::Translations::default(),
            chats: chat::Chats::default(),
            lists: lists::Lists::default(),
            live: live::Live::default(),
            atstore: atstore::Store::default(),
            video: None,
            drafts: drafts::Drafts::default(),
            articles: articles::Articles::default(),
            music: None,
            archive: crate::storage::archive::Archive::default(),
            offline: false,
            collections: collections::Collections::load(),
            privacy: privacy::Privacy::default(),
            stats: stats::Mine::default(),
            sign_in_abort: None,
            toast: None,
            toast_seq: 0,
            session: 0,
            alerts: crate::features::alerts::Alerts::default(),
            notifier: Arc::new(crate::platform::notify::System),
            tray: tray::TrayState::default(),
            update: crate::features::settings::UpdateCheck::Off,
        };
        app.images.set_lazy(app.settings.data_saver);
        std::thread::spawn(image_cache::trim);
        let task = Task::batch([
            app.restore_task(),
            theme::system::current().map(|a| Message::Window(window::Msg::SystemTheme(a))),
        ]);
        (app, task)
    }

    // ---- Update --------------------------------------------------------------------

    fn update(&mut self, message: Message) -> Task<Message> {
        let message = match message {
            Message::Session(session, inner) if session == self.session => *inner,
            Message::Session(..) => return Task::none(),
            other => other,
        };
        let before = self.toast.clone();
        let mut task = self.handle(message);
        self.sync_tray();
        // Images that were drawn while dropped or due a retry.
        let wanted = self.images.take_wanted();
        if !wanted.is_empty() {
            task = Task::batch([task, self.fetch_images(wanted)]);
        }
        if self.toast.is_some() && self.toast != before {
            self.toast_seq += 1;
            let seq = self.toast_seq;
            task = Task::batch([
                task,
                Task::perform(tokio::time::sleep(TOAST_TIME), move |()| {
                    Message::ToastExpired(seq)
                }),
            ]);
        }
        // Tag everything this started with the session it belongs to.
        let session = self.session;
        task.map(move |m| Message::Session(session, Box::new(m)))
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Account(msg) => self.account_update(msg),
            Message::Login(msg) => self.login_update(msg),
            Message::Prefs(msg) => self.prefs_update(msg),
            Message::Nav(msg) => self.nav_update(msg),
            Message::Timeline(msg) => self.timeline_update(msg),
            Message::People(msg) => self.people_update(msg),
            Message::Feeds(msg) => self.feeds_update(msg),
            Message::Moderation(msg) => self.moderation_update(msg),
            Message::Thread(msg) => self.thread_update(msg),
            Message::Profile(msg) => self.profile_update(msg),
            Message::Notifications(msg) => self.notifications_update(msg),
            Message::Settings(msg) => self.settings_update(msg),
            Message::Tray(msg) => self.tray_update(msg),
            Message::Updates(msg) => self.updates_update(msg),
            Message::Alerts(msg) => self.alerts_update(msg),
            Message::UnreadTick => {
                // Also the regular moment to keep caches in check, not only on navigation,
                // and to keep what was loaded.
                self.keep_loaded(false);
                self.trim_caches();
                if self.win.focused {
                    return self.notifications_update(notifications::Msg::Count);
                }
                // In the background, also look for anything to pop up. When that fetches
                // your notifications, the unread count comes from them.
                let alerts = self.alerts_update(crate::features::alerts::Msg::Check);
                if self.alerts_fetch().0 {
                    return alerts;
                }
                Task::batch([alerts, self.notifications_update(notifications::Msg::Count)])
            }
            Message::Pictures(msg) => self.pictures_update(msg),
            Message::Viewer(msg) => self.viewer_update(msg),
            Message::Window(msg) => self.window_update(msg),
            Message::Popup(msg) => self.popup_update(msg),
            Message::Action(msg) => self.action_update(msg),
            Message::Compose(msg) => self.compose_update(msg),
            Message::Chat(msg) => self.chat_update(msg),
            Message::Lists(msg) => self.lists_update(msg),
            Message::Live(msg) => self.live_update(msg),
            Message::Atstore(msg) => self.atstore_update(msg),
            Message::Deck(msg) => self.deck_update(msg),
            Message::Filter(msg) => self.filters_update(msg),
            Message::Translate(msg) => self.translate_update(msg),
            Message::Video(msg) => self.video_update(msg),
            Message::Interaction(msg) => self.interaction_update(msg),
            Message::Drafts(msg) => self.drafts_update(msg),
            Message::Articles(msg) => self.articles_update(msg),
            Message::Music(msg) => self.music_update(msg),
            Message::Collections(msg) => self.collections_update(msg),
            Message::Privacy(msg) => self.privacy_update(msg),
            Message::Stats(msg) => self.stats_update(msg),
            Message::Composer(msg) => self.composer_update(msg),
            Message::Search(msg) => self.search_update(msg),
            Message::DeckHint(take) => {
                self.settings.deck_hint_seen = true;
                self.save_settings();
                if take {
                    self.handle(Message::Deck(deck::Msg::Toggle(true)))
                } else {
                    Task::none()
                }
            }
            Message::SaveFailed(failed) => {
                self.toast = Some(failed.to_string());
                Task::none()
            }
            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }
            Message::ToastExpired(seq) => {
                if seq == self.toast_seq {
                    self.toast = None;
                }
                Task::none()
            }
            Message::Session(session, inner) if session == self.session => self.handle(*inner),
            Message::Session(..) => Task::none(),
        }
    }

    // ---- Post and actor state ----------------------------------------------------

    // ---- Images --------------------------------------------------------------------
}

/// How long a notice stays up before it fades on its own.
const TOAST_TIME: Duration = Duration::from_secs(6);
