//! The app's side of what you do to posts and accounts (`features::actions`): passing
//! it messages, and applying each change it reports to every cached copy.

use super::{App, Message, Route, TimelineKey, timeline};
use crate::api::AuthorFilter;
pub use crate::features::actions::{Actions, Msg};
use crate::features::actions::{Event, Known};
use crate::model::{ActorViewer, AtUri, Did, PostView, StrongRef};
use iced::Task;
use std::collections::HashMap;

/// Posts taken out while their delete is saved, to put back if it is refused.
pub(super) type Deleting = HashMap<AtUri, timeline::Removed>;

impl Known for App {
    fn post(&self, uri: &AtUri) -> Option<PostView> {
        self.find_post(uri)
    }

    fn actor(&self, did: &Did) -> Option<ActorViewer> {
        self.actor_viewer(did)
    }

    fn pinned(&self) -> Option<StrongRef> {
        self.me.as_ref().and_then(|m| m.pinned_post.clone())
    }
}

impl App {
    pub(super) fn action_update(&mut self, msg: Msg) -> Task<Message> {
        // "Delete post" keeps the menu open so "Really delete?" can be pressed.
        if !matches!(
            msg,
            Msg::Like(..)
                | Msg::Bookmark(..)
                | Msg::Created(..)
                | Msg::Done(..)
                | Msg::AskDelete(..)
        ) {
            self.popups.menu = None;
        }
        let mut actions = std::mem::take(&mut self.actions);
        let out = actions.update(msg, &*self, &self.client);
        self.actions = actions;
        let mut tasks = vec![out.task.map(Message::Action)];
        for event in out.events {
            tasks.push(self.action_event(event));
        }
        Task::batch(tasks)
    }

    fn action_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Post(uri, change) => self.with_post(&uri, |p| change.apply(p)),
            Event::Thread(root, change) => self.with_thread(&root, |p| change.apply(p)),
            Event::Actor(did, change) => self.with_actor(&did, |v| change.apply(v)),
            Event::Remove(uri) => {
                let removed = self.timelines.remove_post(&uri);
                self.threads.remove_post(&uri);
                if self.route() == &Route::Thread(uri.clone()) {
                    self.routes.pop();
                }
                self.deleting.insert(uri, removed);
            }
            Event::Removed(uri) => {
                self.deleting.remove(&uri);
            }
            Event::PutBack(uri) => {
                if let Some(removed) = self.deleting.remove(&uri) {
                    self.timelines.restore(removed);
                }
            }
            Event::ShowPinned(pin) => self.show_pinned(pin),
            Event::PinSaved => {
                self.toast = Some(
                    if self.me.as_ref().is_some_and(|m| m.pinned_post.is_some()) {
                        "Pinned to your profile."
                    } else {
                        "Unpinned from your profile."
                    }
                    .into(),
                );
                // Your posts list shows the pinned post first, so it is fetched again.
                let me = (
                    self.my_did.clone(),
                    self.me.as_ref().map(|m| m.handle.clone()),
                );
                self.timelines.retain(|k| {
                    !matches!(k, TimelineKey::Author(a, AuthorFilter::Posts)
                        if *a == me.0 || Some(a) == me.1.as_ref())
                });
                return self.load_current(false);
            }
            Event::Toast(t) => self.toast = Some(t),
            Event::Failed(action, e) => return self.action_failed(action, e),
        }
        Task::none()
    }

    /// Shows `pin` as the post pinned to your profile, wherever your profile and
    /// posts are cached.
    fn show_pinned(&mut self, pin: Option<StrongRef>) {
        let me = self.my_did.clone();
        if let Some(m) = &mut self.me {
            m.pinned_post.clone_from(&pin);
        }
        self.profile.set_pinned(&me, pin.as_ref());
        let uri = pin.map(|p| p.uri);
        self.each_post(|p| {
            if p.author.did == me {
                p.viewer.pinned = uri.as_deref() == Some(p.uri.as_str());
            }
        });
    }
}
