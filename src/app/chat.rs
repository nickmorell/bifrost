//! The app's side of the chat feature (`features::chat`): passing it messages, doing
//! what its events ask, and placing its views on screens.

use super::{App, Message, Route, Toggle, account, actions, nav, popups};
use crate::api::{ApiError, ReportSubject};
use crate::features::Update;

pub use crate::features::chat::{Chats, LIST_POLL, Msg, OPEN_POLL, QUIET_POLL};
use crate::features::chat::{Ctx, Event, Host, NewChat, Share, View};
use crate::model::{PostRecord, QuotedPost};
use crate::theme::size;
use iced::{Element, Task};

impl App {
    pub(super) fn chat_update(&mut self, msg: Msg) -> Task<Message> {
        let route = self.route().clone();
        let ctx = Ctx {
            me: &self.my_did,
            open: match &route {
                Route::Chat(id) => Some(id.as_str()),
                _ => None,
            },
        };
        let out = self.chats.update(msg, &ctx, &self.client);
        self.chats_done(out)
    }

    /// Runs the feature's task and does what its events ask, in order.
    fn chats_done(&mut self, out: Update<Msg, Event>) -> Task<Message> {
        let mut tasks = vec![out.task.map(Message::Chat)];
        for event in out.events {
            tasks.push(match event {
                Event::OpenChat(id) => self.push_route(Route::Chat(id)),
                Event::OpenChats => self.push_route(Route::Chats),
                Event::Back => self.handle(Message::Nav(nav::Msg::Back)),
                Event::SessionExpired => self.handle_session_error(ApiError::SessionExpired),
                Event::Toast(t) => {
                    self.toast = Some(t);
                    Task::none()
                }
                Event::Images(urls) => self.request_images(urls),
                Event::CloseMenu => {
                    self.popups.menu = None;
                    Task::none()
                }
                Event::OpenProfile(did) => {
                    self.handle(Message::Nav(nav::Msg::OpenProfile(did.to_string())))
                }
                Event::CopyText(t) => self.handle(Message::Popup(popups::Msg::CopyText(t))),
                Event::Block(did) => {
                    self.handle(Message::Action(actions::Msg::Block(did, Toggle::On)))
                }
                Event::Report(did) => self.handle(Message::Moderation(
                    super::moderation::Msg::Report(ReportSubject::Account(did)),
                )),
                Event::SignOut => self.handle(Message::Account(account::Msg::Logout(
                    self.my_did.to_string(),
                ))),
                Event::OpenUrl(url) => self.handle(Message::Nav(nav::Msg::OpenUrl(url))),
                Event::Refresh => self.handle(Message::Nav(nav::Msg::Refresh)),
                Event::Retry => self.handle(Message::Nav(nav::Msg::Retry)),
                Event::SignInAgain => self.handle(Message::Account(account::Msg::Changed(Ok(())))),
                Event::ListScrolled(v) => {
                    self.handle(Message::Nav(nav::Msg::Scrolled("chats".into(), v)))
                }
            });
        }
        Task::batch(tasks)
    }

    /// Loads what the chat list, a chat or Settings needs when it is shown.
    pub(super) fn chat_enter(&mut self) -> Task<Message> {
        match self.route().clone() {
            Route::Chats => self.chats.enter_list(&self.client).map(Message::Chat),
            Route::Chat(id) => {
                let ctx = Ctx {
                    me: &self.my_did,
                    open: Some(&id),
                };
                let out = self.chats.enter_chat(&ctx, &self.client);
                self.chats_done(out)
            }
            Route::Settings => self.chats.enter_settings(&self.client).map(Message::Chat),
            _ => Task::none(),
        }
    }

    pub(super) fn chat_refresh(&mut self) -> Task<Message> {
        match self.route().clone() {
            Route::Chats => self.chats.refresh_list(&self.client).map(Message::Chat),
            Route::Chat(id) => {
                let ctx = Ctx {
                    me: &self.my_did,
                    open: Some(&id),
                };
                let out = self.chats.refresh_chat(&ctx, &self.client);
                self.chats_done(out)
            }
            _ => Task::none(),
        }
    }

    pub(super) fn chat_title(&self, id: &str) -> String {
        self.chats.title(id, &self.my_did)
    }

    // ---- Views ---------------------------------------------------------------------

    fn chat_views(&self) -> View<'_, Self, Message> {
        View {
            chats: &self.chats,
            host: self,
            me: &self.my_did,
            lift: Message::Chat,
        }
    }

    pub(super) fn chat_bar_actions<'a>(&'a self, id: &'a str) -> Element<'a, Message> {
        self.chat_views().bar_actions(id)
    }

    pub(super) fn chats_view(&self) -> Element<'_, Message> {
        self.chat_views().list()
    }

    pub(super) fn chat_view<'a>(&'a self, id: &'a str) -> Element<'a, Message> {
        self.chat_views().chat(id)
    }

    pub(super) fn new_chat_dialog<'a>(&'a self, n: &'a NewChat) -> Element<'a, Message> {
        self.chat_views().new_chat(n)
    }

    pub(super) fn share_dialog<'a>(&'a self, s: &'a Share) -> Element<'a, Message> {
        self.chat_views().share(s)
    }

    /// The "Who can message you" setting.
    pub(super) fn chat_settings(&self) -> Element<'_, Message> {
        self.chat_views().settings()
    }
}

impl Host<Message> for App {
    fn rich_text<'a>(&'a self, body: &'a PostRecord) -> Element<'a, Message> {
        super::post::post_text(self.look(), body, size::BODY)
    }

    fn quoted_post<'a>(&'a self, post: &'a QuotedPost) -> Element<'a, Message> {
        super::post::quoted_post(self, post, 1)
    }

    fn browser_sign_in(&self) -> bool {
        self.accounts_list()
            .iter()
            .any(|a| a.did == self.my_did && a.oauth)
    }
}
