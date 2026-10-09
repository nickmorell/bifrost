//! Loading notifications, the unseen count, and marking them seen.

use super::{Ctx, Event, Msg, Notifications, Out};
use crate::api::ApiError;
use crate::api::service::NotificationsApi;
use crate::features::Update;
use crate::model::{AtUri, Notification, Page, PostView};
use iced::Task;
use std::collections::HashSet;

impl Notifications {
    pub fn update(&mut self, msg: Msg, ctx: &Ctx, api: &impl NotificationsApi) -> Out {
        match msg {
            Msg::Open { force } => {
                let refresh = force || self.is_stale();
                let seen = self.mark_seen(api);
                if refresh {
                    let load = self.load(false, api);
                    Update::task(Task::batch([load, seen]))
                } else {
                    Update::task(seen)
                }
            }
            Msg::Load { append } => Update::task(self.load(append, api)),
            Msg::Loaded(append, result) => {
                self.loading = false;
                match result {
                    Ok((page, posts)) => self.loaded(append, page, posts),
                    Err(e) => Update::event(Event::Failed(e)),
                }
            }
            Msg::Filter(filter) => {
                self.filter = filter;
                self.regroup();
                Update::event(Event::ScrollToTop)
            }
            Msg::Measured(row, height) => {
                self.heights.set(row, height);
                Update::none()
            }
            Msg::Expand(uri) => {
                if !self.expanded.remove(&uri) {
                    self.expanded.insert(uri);
                }
                Update::none()
            }
            Msg::Count => {
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.unread_count().await },
                    Msg::Counted,
                ))
            }
            Msg::Counted(Ok(n)) => {
                if n <= self.unread {
                    self.unread = n;
                    Update::none()
                } else if ctx.showing {
                    // The list is open: pull the new ones in and mark them seen.
                    self.unread = 0;
                    let load = self.load(false, api);
                    Update::task(Task::batch([load, Self::seen_task(api)]))
                } else {
                    // Opening the list refreshes it while there are unseen ones.
                    self.unread = n;
                    Update::event(Event::Arrived)
                }
            }
            // A failed background poll is not worth interrupting anyone for.
            Msg::Counted(Err(_)) | Msg::Seen(Ok(())) => Update::none(),
            Msg::Seen(Err(e)) => Update::event(Event::Quiet(e)),
            Msg::OpenProfile(did) => Update::event(Event::OpenProfile(did)),
            Msg::OpenThread(uri) => Update::event(Event::OpenThread(uri)),
            Msg::FollowBack(did) => Update::event(Event::FollowBack(did)),
        }
    }

    /// Marks everything seen, if anything is unseen.
    fn mark_seen(&mut self, api: &impl NotificationsApi) -> Task<Msg> {
        if self.unread == 0 {
            return Task::none();
        }
        self.unread = 0;
        Self::seen_task(api)
    }

    fn seen_task(api: &impl NotificationsApi) -> Task<Msg> {
        let api = api.clone();
        Task::perform(
            async move { api.mark_notifications_seen().await },
            Msg::Seen,
        )
    }

    fn load(&mut self, append: bool, api: &impl NotificationsApi) -> Task<Msg> {
        self.loading = true;
        let cursor = if append { self.cursor.clone() } else { None };
        let known: HashSet<AtUri> = self.subjects.keys().cloned().collect();
        let api = api.clone();
        Task::perform(page_with_posts(api, cursor, known), move |r| {
            Msg::Loaded(append, r)
        })
    }

    fn loaded(&mut self, append: bool, page: Page<Notification>, posts: Vec<PostView>) -> Out {
        let avatars = page
            .items
            .iter()
            .filter_map(|n| n.author.avatar.clone())
            .collect();
        let uris = posts.iter().map(|p| p.uri.clone()).collect();
        for p in posts {
            self.subjects.insert(p.uri.clone(), p);
        }
        if append {
            self.items.extend(page.items);
            self.cursor = page.cursor;
        } else if self.items.is_empty() {
            self.items = page.items;
            self.cursor = page.cursor;
        } else {
            // A refresh: put new arrivals on top and keep the pages already loaded
            // below, so the list does not jump back to one page.
            let fresh: HashSet<&str> = page.items.iter().map(|n| n.uri.as_str()).collect();
            let older: Vec<Notification> = self
                .items
                .drain(..)
                .filter(|n| !fresh.contains(n.uri.as_str()))
                .collect();
            self.items = page.items;
            self.items.extend(older);
        }
        self.loaded = true;
        self.regroup();
        Update::event(Event::Loaded {
            avatars,
            posts: uris,
        })
    }
}

/// A page of notifications with the posts they are about that aren't `known` yet.
pub(super) async fn page_with_posts(
    api: impl NotificationsApi,
    cursor: Option<String>,
    known: HashSet<AtUri>,
) -> Result<(Page<Notification>, Vec<PostView>), ApiError> {
    let page = api.notifications(cursor).await?;
    let mut uris: Vec<AtUri> = page
        .items
        .iter()
        .filter_map(|n| n.subject_uri().cloned())
        .filter(|u| !known.contains(u))
        .collect();
    uris.sort();
    uris.dedup();
    let posts = api.posts(&uris).await?;
    Ok((page, posts))
}
