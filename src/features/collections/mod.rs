//! Collections: your own folders of posts, kept only on this computer. The
//! Collections screen lists them and makes new ones; a collection's page shows its
//! posts, fetched again from Bluesky.
//!
//! The collections themselves are kept by the app (`storage::collections`), which
//! hands them to [`update`] and the views and applies the [`Event`]s that change them.

mod view;

#[cfg(test)]
mod tests;

pub use view::{Host, List, Page};

use crate::api::ApiError;
use crate::api::service::CollectionsApi;
use crate::features::Update;
use crate::model::{AtUri, Collection, PostView};
use iced::Task;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Msg {
    /// The new collection's name was edited.
    Name(String),
    Create,
    /// Start a new collection with this post in it.
    NewWith(AtUri),
    Open(u64),
    Delete(u64),
    /// Add a post to a collection, or take it out.
    Toggle(u64, AtUri),
    /// Fetch a collection's posts.
    Load(u64),
    Loaded(u64, Result<Vec<PostView>, ApiError>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Create(String, Option<AtUri>),
    Delete(u64),
    Toggle(u64, AtUri),
    /// Show the Collections screen.
    Show,
    /// Show a collection's page.
    Open(u64),
    /// Posts just fetched, for their pictures.
    Posts(Vec<AtUri>),
}

pub type Out = Update<Msg, Event>;

#[derive(Debug, Default)]
pub struct Screen {
    name: String,
    /// The post a new collection will start with.
    pending: Option<AtUri>,
    /// Each collection's posts as last fetched, or why they couldn't be.
    posts: HashMap<u64, Result<Vec<PostView>, String>>,
}

impl Screen {
    /// The fetched posts of a collection, in its order.
    pub fn posts(&self, id: u64) -> Option<&Result<Vec<PostView>, String>> {
        self.posts.get(&id)
    }

    pub fn update(&mut self, msg: Msg, all: &[Collection], api: &impl CollectionsApi) -> Out {
        match msg {
            Msg::Name(name) => {
                self.name = name;
                Update::none()
            }
            Msg::Create => {
                if self.name.trim().is_empty() {
                    return Update::none();
                }
                let name = std::mem::take(&mut self.name);
                Update::event(Event::Create(name, self.pending.take()))
            }
            Msg::NewWith(uri) => {
                self.pending = Some(uri);
                Update::event(Event::Show)
            }
            Msg::Open(id) => Update::event(Event::Open(id)),
            Msg::Delete(id) => {
                self.posts.remove(&id);
                Update::event(Event::Delete(id))
            }
            Msg::Toggle(id, uri) => {
                if let Some(Ok(posts)) = self.posts.get_mut(&id) {
                    posts.retain(|p| p.uri != uri);
                }
                Update::event(Event::Toggle(id, uri))
            }
            Msg::Load(id) => {
                let Some(c) = all.iter().find(|c| c.id == id) else {
                    return Update::none();
                };
                if c.posts.is_empty() {
                    return self.update(Msg::Loaded(id, Ok(Vec::new())), all, api);
                }
                let (api, uris) = (api.clone(), c.posts.clone());
                Update::task(Task::perform(
                    async move { api.posts(&uris).await },
                    move |r| Msg::Loaded(id, r),
                ))
            }
            Msg::Loaded(id, result) => match result {
                Ok(posts) => {
                    let uris = posts.iter().map(|p| p.uri.clone()).collect();
                    self.posts.insert(id, Ok(posts));
                    Update::event(Event::Posts(uris))
                }
                Err(e) => {
                    self.posts.insert(id, Err(e.friendly()));
                    Update::none()
                }
            },
        }
    }

    /// Every fetched post.
    pub fn all_posts(&self) -> impl Iterator<Item = &PostView> {
        self.posts
            .values()
            .filter_map(|r| r.as_ref().ok())
            .flatten()
    }

    /// Every fetched post, to update in place.
    pub fn posts_mut(&mut self) -> impl Iterator<Item = &mut PostView> {
        self.posts
            .values_mut()
            .filter_map(|r| r.as_mut().ok())
            .flatten()
    }
}
