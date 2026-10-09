//! Opening the composer for a new post, a reply, a quote or a draft, and what
//! happens once a post is sent.

use super::composer::Composer;
use super::{App, Message, Route, TimelineKey, timeline};
use crate::api::AuthorFilter;
use crate::model::{AtUri, FeedSource};
use iced::Task;

#[derive(Debug, Clone)]
pub enum Msg {
    New,
    Reply(AtUri),
    Quote(AtUri),
}

impl App {
    pub(super) fn compose_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Reply(uri) => {
                let Some(post) = self.find_post(&uri) else {
                    return Task::none();
                };
                self.stash_composer();
                self.composer = Some(self.new_composer(Composer::reply(post)));
                Task::none()
            }
            Msg::Quote(uri) => {
                self.popups.menu = None;
                let Some(post) = self.find_post(&uri) else {
                    return Task::none();
                };
                self.stash_composer();
                self.composer = Some(self.new_composer(Composer::quote(post)));
                Task::none()
            }
            Msg::New => {
                if self.composer.is_none() {
                    self.composer = Some(self.new_composer(Composer::default()));
                }
                Task::none()
            }
        }
    }

    /// The post was sent: show it where you are.
    pub(super) fn posted(&mut self, reply_parent: Option<AtUri>) -> Task<Message> {
        self.composer = None;
        self.toast = Some("Posted.".into());
        if let Some(uri) = reply_parent {
            self.with_post(&uri, |p| p.reply_count += 1);
            let thread = self.routes.iter().rev().find_map(|r| match r {
                Route::Thread(t) => Some(t.clone()),
                _ => None,
            });
            thread.map_or_else(Task::none, |t| self.load_thread(t))
        } else {
            // Show the new post in Following and on your own profile, wherever
            // you posted from.
            let mine = TimelineKey::Author(self.my_did.to_string(), AuthorFilter::Posts);
            self.timelines.forget(&mine);
            let home = TimelineKey::Feed(FeedSource::Following);
            let refresh_home =
                self.current_timeline_key() != Some(home.clone()) && self.timelines.contains(&home);
            let current = self.refresh_current();
            if refresh_home {
                let home = self.timeline_update(timeline::Msg::Refresh(home));
                return Task::batch([current, home]);
            }
            current
        }
    }
}
