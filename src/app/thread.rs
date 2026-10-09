//! The app's side of the thread feature (`features::thread`): passing it messages,
//! doing what its events ask, and drawing its posts.

use super::pictures::post_images;
use super::post::{self, Opts, Size};
use super::{App, Message, Route};
use crate::features::thread::{Event, Host, Shape, View};

pub use crate::features::thread::{Msg, Threads};
use crate::model::{AtUri, PostView};
use crate::moderation::Context;
use iced::{Element, Task};

impl App {
    pub(super) fn thread_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.threads.update(msg, &self.my_did, &self.client);
        let mut tasks = vec![out.task.map(Message::Thread)];
        for event in out.events {
            tasks.push(match event {
                Event::Loaded(uri) => self.thread_images(&uri),
                Event::Failed(e) => self.handle_session_error(e),
                Event::Open(uri) => self.push_route(Route::Thread(uri)),
            });
        }
        Task::batch(tasks)
    }

    pub(super) fn load_thread(&mut self, uri: AtUri) -> Task<Message> {
        self.thread_update(Msg::Load(uri))
    }

    /// Loads the pictures of a thread's posts.
    fn thread_images(&mut self, uri: &str) -> Task<Message> {
        let posts = self.threads.posts_of(uri);
        let mut urls = Vec::new();
        for p in &posts {
            post_images(p, &mut urls);
        }
        for p in posts {
            self.integration_images(p, &mut urls);
        }
        self.request_images(urls)
    }

    pub(super) fn thread_view<'a>(&'a self, uri: &str) -> Element<'a, Message> {
        View {
            threads: &self.threads,
            host: self,
            window: self.window(&Route::Thread(AtUri::from(uri)).scroll_id()),
            lift: Message::Thread,
        }
        .screen(uri)
    }
}

impl Host<Message> for App {
    fn post<'a>(&'a self, post: &'a PostView, shape: Shape) -> Element<'a, Message> {
        post::view(
            self,
            post,
            Opts {
                ctx: Context::Thread,
                size: if shape.focused {
                    Size::Focused
                } else {
                    Size::Normal
                },
                line_above: shape.line_above,
                line_below: shape.line_below,
                ..Opts::default()
            },
        )
    }

    fn reply_note(&self, root: &PostView) -> Option<String> {
        super::interaction::reply_note(root)
    }

    fn retry<'a>(&self, why: String) -> Element<'a, Message> {
        super::util::retry(why)
    }

    fn scroll<'a>(&self, content: Element<'a, Message>) -> Element<'a, Message> {
        App::scroll(self, content)
    }
}
