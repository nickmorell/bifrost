//! Signing in and out: starting a session for an account, switching and adding
//! accounts, and what happens when the server says the session has ended.

use super::login::Login;
use super::{
    App, Message, Phase, Route, TimelineKey, chat, deck, lists, notifications, prefs, search,
};
use crate::api::ApiError;
use crate::model::{Did, FeedSource, Preferences, Profile};
use iced::Task;

/// Signing in and out.
#[derive(Debug, Clone)]
pub enum Msg {
    /// The saved session was restored, or there was none.
    Booted(Result<String, ApiError>),
    LoggedIn(Result<String, ApiError>),
    MeLoaded(Box<Result<Profile, ApiError>>),
    Switch(String),
    Logout(String),
    Add,
    /// The signed-in accounts changed: start again with the current one.
    Changed(Result<(), ApiError>),
}

impl App {
    pub(super) fn account_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Booted(Ok(did)) | Msg::LoggedIn(Ok(did)) => self.start_session(did),
            Msg::Booted(Err(_)) => {
                self.account_list = self.accounts.accounts().unwrap_or_default();
                self.phase = Phase::Login(Login::default());
                Task::none()
            }
            Msg::LoggedIn(Err(e)) => {
                if let Phase::Login(state) = &mut self.phase {
                    state.failed(&e);
                }
                Task::none()
            }
            Msg::MeLoaded(r) => match *r {
                Ok(me) => {
                    // Only the avatar shows straight away; the banner loads with the profile.
                    let task = self.request_images(me.avatar.clone());
                    self.me = Some(me);
                    task
                }
                Err(e) => self.handle_error(e),
            },
            Msg::Switch(did) => match self.accounts.switch(&did) {
                Ok(()) => self.reset_and_restore(),
                Err(e) => {
                    self.toast = Some(e.to_string());
                    Task::none()
                }
            },
            Msg::Logout(did) => {
                // What BiFrost kept for this account goes with it.
                if let Err(e) = self.drafts.store.delete_account(&did) {
                    self.toast = Some(format!("Could not delete this account's drafts: {e}"));
                }
                self.settings.forget_account(&did);
                self.save_settings();
                self.reading.places.forget_account(&did);
                self.reading.places.save(false);
                self.collections.store.forget_account(&did);
                self.collections.store.save();
                if self.archive.account() == did.as_str() {
                    self.archive = crate::storage::archive::Archive::default();
                }
                crate::storage::archive::Archive::forget(&did);
                let client = self.client.clone();
                Task::perform(async move { client.logout(&did).await }, |a| {
                    Message::Account(Msg::Changed(a))
                })
            }
            Msg::Changed(result) => {
                if let Err(e) = result {
                    self.toast = Some(e.friendly());
                }
                self.reset_and_restore()
            }
            Msg::Add => {
                self.phase = Phase::Login(Login::adding_account());
                Task::none()
            }
        }
    }

    pub(super) fn restore_task(&self) -> Task<Message> {
        let accounts = self.accounts.clone();
        Task::perform(
            async move {
                accounts
                    .restore()
                    .await
                    .map(|s| s.did)
                    .map_err(ApiError::from)
            },
            |a| Message::Account(Msg::Booted(a)),
        )
    }

    pub(super) fn start_session(&mut self, did: String) -> Task<Message> {
        self.clear_caches();
        self.start_tray();
        let seen = self.alerts_seen();
        self.account_list = self.accounts.accounts().unwrap_or_default();
        self.my_did = Did::from(did.as_str());
        self.open_archive();
        self.drafts.list = self.drafts.store.list(&did);
        let articles = self.articles_update(super::articles::Msg::Load);
        let update = self.updates_update(super::updates::Msg::Check);
        self.phase = Phase::Ready;
        self.routes = vec![Route::Home];
        self.feeds.following_only();
        let (c1, c2, c3) = (
            self.client.clone(),
            self.client.clone(),
            self.client.clone(),
        );
        // Following loads alongside the preferences rather than after them, and is
        // filtered once they arrive. Chats wait for the first posts to show.
        self.chats_deferred = true;
        let following = self.load_key(TimelineKey::Feed(FeedSource::Following), false);
        Task::batch([
            seen,
            articles,
            update,
            Task::perform(async move { c1.preferences().await }, |a| {
                Message::Prefs(prefs::Msg::Loaded(a))
            }),
            following,
            Task::perform(async move { Box::pin(c2.profile(&did)).await }, |a| {
                Message::Account(Msg::MeLoaded(Box::new(a)))
            }),
            Task::perform(async move { c3.unread_count().await }, |r| {
                Message::Notifications(notifications::Msg::Counted(r))
            }),
        ])
    }

    pub(super) fn clear_caches(&mut self) {
        self.stash_composer();
        // The account's posts are kept before they are dropped.
        self.keep_loaded(true);
        // Anything still loading for the old session is dropped when it arrives.
        self.session += 1;
        self.scroll_offsets.clear();
        self.selected = None;
        self.me = None;
        self.prefs = Preferences::default();
        self.timelines.clear();
        self.threads.clear();
        self.profile.clear();
        self.people.clear();
        self.pref_saver.clear();
        self.moderation.clear();
        self.feeds.forget();
        self.notifications = notifications::Notifications::default();
        self.alerts = crate::features::alerts::Alerts::default();
        self.search = search::Search::default();
        self.revealed.clear();
        self.popups.menu = None;
        self.actions.cancel_delete();
        self.composer = None;
        self.interaction_edit = None;
        self.chats = chat::Chats::default();
        self.lists = lists::Lists::default();
        self.articles = super::articles::Articles::default();
        self.live.clear();
        self.deck = deck::Deck::default();
        self.translations.clear();
        self.video = None;
        self.music = None;
        self.viewer = None;
    }

    pub(super) fn reset_and_restore(&mut self) -> Task<Message> {
        self.clear_caches();
        self.phase = Phase::Booting;
        self.restore_task()
    }

    /// Something went wrong talking to the server. An expired session sends the user
    /// back to the login screen; anything else is shown as a dismissible notice.
    pub(super) fn handle_error(&mut self, e: ApiError) -> Task<Message> {
        if !matches!(e, ApiError::SessionExpired) {
            self.toast = Some(e.friendly());
        }
        self.handle_session_error(e)
    }

    /// Reports a write the server refused, saying what did not happen.
    pub(super) fn action_failed(&mut self, action: &str, e: ApiError) -> Task<Message> {
        if !matches!(e, ApiError::SessionExpired) {
            self.toast = Some(format!("Couldn't {action}. {}", e.friendly()));
        }
        self.handle_session_error(e)
    }

    /// Sends the user back to the login screen if the session has ended.
    pub(super) fn handle_session_error(&mut self, e: ApiError) -> Task<Message> {
        if matches!(e, ApiError::Unreachable(_)) {
            self.offline = true;
        }
        if matches!(e, ApiError::SessionExpired) {
            self.clear_caches();
            self.phase = Phase::Login(Login::expired());
        }
        Task::none()
    }

    /// Fetches your profile again after it changed, here and on your profile screen.
    pub(super) fn reload_me(&mut self) -> Task<Message> {
        let me = self.my_did.clone();
        self.profile.forget_account(&me);
        let client = self.client.clone();
        let fetch = Task::perform(async move { Box::pin(client.profile(&me)).await }, |a| {
            Message::Account(Msg::MeLoaded(Box::new(a)))
        });
        Task::batch([fetch, self.on_enter()])
    }
}
