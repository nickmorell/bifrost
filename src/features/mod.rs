//! One folder per feature: its state, messages, update and views.
//!
//! A feature is driven by the app shell. It gets its own messages, calls Bluesky
//! through its service trait in [`crate::api::service`], and reports what the shell
//! has to do (open a page, show a toast, load pictures) as events. Its views return
//! elements of its own message type, which the shell maps into the app's.

pub mod actions;
pub mod alerts;
pub mod articles;
pub mod chat;
pub mod collections;
pub mod composer;
pub mod deck;
pub mod drafts;
pub mod feeds;
pub mod lists;
pub mod live;
pub mod login;
pub mod moderation;
pub mod notifications;
pub mod people;
pub mod prefs;
pub mod privacy;
pub mod profile;
pub mod search;
pub mod settings;
pub mod stats;
pub mod thread;
pub mod timeline;
pub mod translate;

use crate::components::Picture;
use iced::Task;

/// What a feature's views need from the app around them.
pub trait Pictures {
    /// The picture at `url` as far as it has got, or `None` when there is none or it
    /// failed to load.
    fn picture(&self, url: Option<&str>) -> Option<Picture>;
}

/// What a feature's update gives back: work to run and things for the shell to do,
/// in order.
#[must_use]
pub struct Update<M, E> {
    pub task: Task<M>,
    pub events: Vec<E>,
}

impl<M, E> Update<M, E> {
    pub fn none() -> Self {
        Self::task(Task::none())
    }

    pub fn task(task: Task<M>) -> Self {
        Self {
            task,
            events: Vec::new(),
        }
    }

    pub fn event(event: E) -> Self {
        Self::none().and(event)
    }

    /// Adds an event after the ones already there.
    pub fn and(mut self, event: E) -> Self {
        self.events.push(event);
        self
    }
}

impl<M: Send + 'static, E> Update<M, E> {
    /// Runs `next`'s task alongside this one, and adds its events after these.
    pub fn then(mut self, next: Self) -> Self {
        self.task = Task::batch([self.task, next.task]);
        self.events.extend(next.events);
        self
    }
}
