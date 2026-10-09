//! The app's side of the notifications feature (`features::notifications`): passing it
//! messages, doing what its events ask, and drawing its posts.

use super::pictures::post_images;
use super::post::{self, Opts};
use super::{App, Message, Route, Toggle, actions, nav};
use crate::features::notifications::{Ctx, Event, Host, View};

pub use crate::features::notifications::{Msg, Notifications};
use crate::model::PostView;
use iced::{Element, Task};

impl App {
    pub(super) fn notifications_update(&mut self, msg: Msg) -> Task<Message> {
        // The unread count is asked for every half minute, so it tells whether
        // Bluesky can be reached.
        if let Msg::Counted(r) = &msg {
            self.offline = matches!(r, Err(crate::api::ApiError::Unreachable(_)));
        }
        let ctx = Ctx {
            showing: self.route() == &Route::Notifications && self.composer.is_none(),
        };
        let out = self.notifications.update(msg, &ctx, &self.client);
        let mut tasks = vec![out.task.map(Message::Notifications)];
        for event in out.events {
            tasks.push(match event {
                Event::Loaded { avatars, posts } => {
                    self.offline = false;
                    let mut urls = avatars;
                    for uri in &posts {
                        if let Some(p) = self.notifications.post(uri) {
                            post_images(p, &mut urls);
                            self.integration_images(p, &mut urls);
                        }
                    }
                    self.request_images(urls)
                }
                Event::Failed(e) => self.handle_error(e),
                Event::Quiet(e) => self.handle_session_error(e),
                // Flash the taskbar entry if the window is in the background.
                Event::Arrived => iced::window::latest().and_then(|id| {
                    iced::window::request_user_attention(
                        id,
                        Some(iced::window::UserAttention::Informational),
                    )
                }),
                Event::ScrollToTop => self.scroll_to_top(),
                Event::OpenProfile(did) => {
                    self.handle(Message::Nav(nav::Msg::OpenProfile(did.to_string())))
                }
                Event::OpenThread(uri) => self.handle(Message::Nav(nav::Msg::OpenThread(uri))),
                Event::FollowBack(did) => {
                    self.handle(Message::Action(actions::Msg::Follow(did, Toggle::On)))
                }
            });
        }
        Task::batch(tasks)
    }

    /// Loads notifications if they are stale, and marks them seen.
    pub(super) fn open_notifications(&mut self, force: bool) -> Task<Message> {
        self.notifications_update(Msg::Open { force })
    }

    /// Loads the next older page, if there is one and nothing is loading.
    pub(super) fn more_notifications(&mut self) -> Task<Message> {
        if self.notifications.has_more() {
            self.notifications_update(Msg::Load { append: true })
        } else {
            Task::none()
        }
    }

    /// The notifications with their filter, unscrolled, for the screen or a deck
    /// column, drawn only near the scroll position of `scroll_id`.
    pub(super) fn notifications_body(&self, scroll_id: &str) -> Element<'_, Message> {
        View {
            notifications: &self.notifications,
            host: self,
            window: self.window(scroll_id),
            lift: Message::Notifications,
        }
        .body()
    }
}

impl Host<Message> for App {
    fn post<'a>(&'a self, post: &'a PostView) -> Element<'a, Message> {
        post::view(self, post, Opts::default())
    }
}
