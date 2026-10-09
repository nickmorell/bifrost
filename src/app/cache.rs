//! The posts, profiles and lists kept in memory: changing a post or account in
//! every copy, finding a post, and dropping what is no longer needed.

use super::{App, Route, TimelineKey, post};
use crate::model::{ActorViewer, PostView};

impl App {
    /// Forgets threads, profiles and timelines that are not on the Back stack once
    /// too many have piled up, so a long session does not keep growing. Anything
    /// dropped is simply loaded again if opened.
    pub(super) fn trim_caches(&mut self) {
        let routes = &self.routes;
        let in_routes = |pred: &dyn Fn(&Route) -> bool| routes.iter().any(pred);
        self.threads.trim(MAX_CACHED, |uri| {
            in_routes(&|r| matches!(r, Route::Thread(u) if u == uri))
        });
        let me = self.my_did.clone();
        let on_profile =
            |did: &str| did == me || in_routes(&|r| matches!(r, Route::Profile(d, _) if d == did));
        self.profile.trim(MAX_CACHED, on_profile);
        if self.people.count() > MAX_CACHED {
            self.people.retain(|did, kind| {
                in_routes(&|r| matches!(r, Route::People(d, k) if d == did && *k == kind))
            });
        }
        if self.timelines.count() > MAX_CACHED {
            let feeds = &self.feeds;
            let deck = self.deck_keys();
            self.timelines.retain(|key| {
                deck.contains(key)
                    || match key {
                        TimelineKey::Bookmarks => true,
                        TimelineKey::Feed(source) => {
                            feeds.is_tab(source)
                                || in_routes(&|r| matches!(r, Route::Feed(s, _) if s == source))
                        }
                        TimelineKey::Author(did, _) => on_profile(did),
                    }
            });
        }
        // Long timelines that are off screen go back to their first few pages.
        let shown = self.current_timeline_key();
        let deck = self.deck_keys();
        self.timelines.shorten(MAX_POSTS, |key| {
            shown.as_ref() == Some(key) || deck.contains(key)
        });
        if self.lists.screens.len() > MAX_CACHED {
            self.lists
                .screens
                .retain(|uri, _| in_routes(&|r| matches!(r, Route::List(u) if u == uri)));
        }
        if self.lists.packs.len() > MAX_CACHED {
            self.lists
                .packs
                .retain(|uri, _| in_routes(&|r| matches!(r, Route::StarterPack(u) if u == uri)));
        }
        if self.lists.actor_packs.len() > MAX_CACHED {
            self.lists.actor_packs.retain(|did, _| on_profile(did));
        }
        if self.chats.threads.len() > MAX_CACHED {
            self.chats
                .threads
                .retain(|id, _| in_routes(&|r| matches!(r, Route::Chat(c) if c == id)));
        }
        // Notifications start over from the newest next time they are opened.
        if !in_routes(&|r| matches!(r, Route::Notifications)) {
            self.notifications.trim(MAX_NOTIFICATIONS);
        }
        if self.translations.len() > MAX_CACHED * 4 {
            self.translations.clear();
        }
        if self.revealed.len() > MAX_CACHED * 20 {
            self.revealed.clear();
        }
        if self.scroll_offsets.len() > MAX_CACHED * 4 {
            let keep = self.scroll_key();
            self.scroll_offsets.retain(|k, _| *k == keep);
        }
        self.feeds.trim(MAX_CACHED * 4);
    }

    /// Applies `f` to every cached copy of the post at `uri`.
    pub(super) fn with_post(&mut self, uri: &str, mut f: impl FnMut(&mut PostView)) {
        self.each_post(|p| {
            if p.uri == uri {
                f(p);
            }
        });
    }

    /// Applies `f` to every cached post in the thread that starts at `root`.
    pub(super) fn with_thread(&mut self, root: &str, mut f: impl FnMut(&mut PostView)) {
        self.each_post(|p| {
            if post::thread_root(p) == root {
                f(p);
            }
        });
    }

    /// Applies `f` to every cached post.
    pub(super) fn each_post(&mut self, mut apply: impl FnMut(&mut PostView)) {
        self.timelines.each_post_mut(&mut apply);
        self.threads.each_post_mut(&mut apply);
        for p in self.notifications.posts_mut() {
            apply(p);
        }
        for p in self.search.posts_mut() {
            apply(p);
        }
        for p in self.collections.screen.posts_mut() {
            apply(p);
        }
    }

    pub(crate) fn find_post(&self, uri: &str) -> Option<PostView> {
        if let Some(p) = self.timelines.posts().find(|p| p.uri == uri) {
            return Some(p.clone());
        }
        if let Some(p) = self.threads.find_post(uri) {
            return Some(p.clone());
        }
        self.notifications
            .post(uri)
            .or_else(|| self.search.post(uri))
            .or_else(|| self.collections.screen.all_posts().find(|p| p.uri == uri))
            .cloned()
    }

    pub(super) fn actor_viewer(&self, did: &str) -> Option<ActorViewer> {
        self.profile.viewer(did).cloned()
    }

    /// Applies `f` to the viewer state of every cached copy of the account `did`.
    pub(super) fn with_actor(&mut self, did: &str, mut f: impl FnMut(&mut ActorViewer)) {
        self.profile.with_actor(did, &mut f);
        let mut posts = |p: &mut PostView| {
            if p.author.did == did {
                f(&mut p.author.viewer);
            }
        };
        for p in self.timelines.posts_mut() {
            posts(p);
        }
        self.threads.each_post_mut(&mut posts);
        for p in self.search.posts_mut() {
            posts(p);
        }
        self.notifications.with_author(did, &mut f);
        for a in self.people.authors_mut().chain(self.search.people_mut()) {
            if a.did == did {
                f(&mut a.viewer);
            }
        }
    }
}

/// Posts kept in a timeline that is off screen.
pub(super) const MAX_POSTS: usize = 300;

/// Threads, profiles or timelines kept around before ones off the Back stack are
/// dropped.
pub(super) const MAX_CACHED: usize = 40;

/// Notifications kept while their screen isn't open.
pub(super) const MAX_NOTIFICATIONS: usize = 300;
