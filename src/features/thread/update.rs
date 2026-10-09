//! Loading threads.

use super::{Event, Msg, Out, Threads, tree};
use crate::api::service::ThreadApi;
use crate::features::Update;
use iced::Task;

impl Threads {
    /// `me` is the signed-in account, whose replies come early.
    pub fn update(&mut self, msg: Msg, me: &str, api: &impl ThreadApi) -> Out {
        match msg {
            Msg::Load(uri) => {
                let api = api.clone();
                let u = uri.clone();
                Update::task(Task::perform(
                    async move { api.thread(&u).await },
                    move |r| Msg::Loaded(uri.clone(), r),
                ))
            }
            Msg::Loaded(uri, Ok(mut node)) => {
                tree::sort(&mut node, me);
                self.loaded.insert(uri.clone(), Ok(node));
                Update::event(Event::Loaded(uri))
            }
            Msg::Loaded(uri, Err(e)) => {
                self.loaded.insert(uri, Err(e.friendly()));
                Update::event(Event::Failed(e))
            }
            Msg::Open(uri) => Update::event(Event::Open(uri)),
            Msg::Measured(row, height) => {
                self.heights.set(row, height);
                Update::none()
            }
        }
    }
}
