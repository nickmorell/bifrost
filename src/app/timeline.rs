//! The app's side of timelines (`features::timeline`): passing them messages, doing
//! what their events ask, and drawing a timeline with the posts the app draws.

use super::pictures::feed_item_images;
use super::post::{self, Opts, Size};
use super::{App, Message, chat};
use crate::features::timeline::{Ctx, Event, Host, Shape, View};

pub use crate::features::timeline::{Key, Msg, Removed, Timeline, Timelines, item_key};
use crate::model::PostView;
use crate::moderation::Context;
use iced::{Element, Task};

impl App {
    /// Handles one timeline message and does what it asks of the app, in order.
    pub(super) fn timeline_update(&mut self, msg: Msg) -> Task<Message> {
        // Chats wait for the first timeline page, so it isn't slowed down.
        let chats = if matches!(msg, Msg::Loaded(..)) && std::mem::take(&mut self.chats_deferred) {
            self.chat_update(chat::Msg::Tick)
        } else {
            Task::none()
        };
        let ctx = Ctx {
            following: &self.prefs.following,
            me: &self.my_did,
            filters: &self.settings.feed_filters,
        };
        let measured = match &msg {
            Msg::Measured(key, ..) => Some(key.clone()),
            _ => None,
        };
        let out = self.timelines.update(msg, &ctx, &self.client);
        let mut tasks = vec![chats, out.task.map(Message::Timeline)];
        if let Some(key) = measured {
            tasks.push(self.hold_place(&key));
        }
        for event in out.events {
            tasks.push(match event {
                Event::Fetched(items) => {
                    self.offline = false;
                    let mut pictures: Vec<String> =
                        items.iter().flat_map(feed_item_images).collect();
                    for item in &items {
                        self.integration_images(&item.post, &mut pictures);
                    }
                    self.request_images(pictures)
                }
                Event::Empty(key) => {
                    if self.current_timeline_key().as_ref() == Some(&key) || self.in_deck(&key) {
                        self.load_more_key(key)
                    } else {
                        Task::none()
                    }
                }
                Event::Found(key, item) => self.go_to_place(key, item),
                Event::Failed(e) => self.handle_session_error(e),
            });
        }
        Task::batch(tasks)
    }

    /// Loads the first page of the current timeline. Cached timelines are kept unless
    /// `force` is set.
    pub(super) fn load_current(&mut self, force: bool) -> Task<Message> {
        match self.current_timeline_key() {
            Some(key) => self.load_key(key, force),
            None => Task::none(),
        }
    }

    /// Loads the first page of a timeline, as `load_current` does.
    pub(super) fn load_key(&mut self, key: Key, force: bool) -> Task<Message> {
        let place = self.find_place(&key);
        Task::batch([place, self.timeline_update(Msg::Load { key, force })])
    }

    /// Loads a timeline's next page, unless it is loading or has no more.
    pub(super) fn load_more_key(&mut self, key: Key) -> Task<Message> {
        self.timeline_update(Msg::More(key))
    }

    /// Puts the new posts found by a background check at the top, returning the task
    /// that fetches their pictures, or `None` if there were none.
    pub(super) fn show_pending(&mut self, key: &Key) -> Option<Task<Message>> {
        let range = self.timelines.show_pending(key)?;
        // Their pictures weren't fetched in the background, in case they were never shown.
        let fresh = self.timelines.get(key)?.items().get(range)?;
        let mut pictures: Vec<String> = fresh.iter().flat_map(feed_item_images).collect();
        for item in fresh {
            self.integration_images(&item.post, &mut pictures);
        }
        Some(self.request_images(pictures))
    }

    /// The part of the scrollable `scroll_id` on screen.
    pub(super) fn window(&self, scroll_id: &str) -> crate::components::Window {
        crate::components::Window {
            offset: self.scroll_offsets.get(scroll_id).copied().unwrap_or(0.0),
            screen: self.layout_size().height.max(400.0),
        }
    }

    /// A timeline, drawn only near the scroll position of `scroll_id`.
    pub(super) fn timeline_view<'a>(
        &'a self,
        key: Option<&Key>,
        ctx: Context,
        scroll_id: &str,
    ) -> Element<'a, Message> {
        View {
            timelines: &self.timelines,
            key: key.cloned(),
            selected: self.selected,
            window: self.window(scroll_id),
            post_id: super::locate::post_id,
            host: Posts { app: self, ctx },
            lift: Message::Timeline,
        }
        .view()
    }
}

/// The app drawing a timeline's posts, moderated for where they are shown.
struct Posts<'a> {
    app: &'a App,
    ctx: Context,
}

impl<'a> Host<'a, Message> for Posts<'a> {
    fn post(&self, p: &'a PostView, shape: Shape<'a>) -> Element<'a, Message> {
        post::view(
            self.app,
            p,
            Opts {
                ctx: self.ctx,
                size: Size::Normal,
                reason: shape.reason,
                reply_parent: shape.reply_parent,
                line_above: shape.line_above,
                line_below: shape.line_below,
                above_reply: shape.above_reply,
            },
        )
    }

    fn hidden(&self, p: &PostView) -> bool {
        self.app.moderate(p, self.ctx).hide.is_some()
    }

    fn retry(&self, why: String) -> Element<'a, Message> {
        super::util::retry(why)
    }
}
