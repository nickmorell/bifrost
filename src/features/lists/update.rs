//! Handling the lists feature's messages for list and pack pages, and loading them.

use super::message::{Ctx, Event, Msg};
use super::state::{ListTab, Lists};
use super::{Out, effects};
use crate::api::ApiError;
use crate::api::service::ListsApi;
use crate::features::Update;
use crate::model::{AtUri, Did, ListItem, ListPage, Mine};
use iced::Task;
use std::collections::HashMap;
use std::time::Duration;

impl Lists {
    /// Handles one message. `api` is cloned into the tasks that need it.
    pub fn update<A: ListsApi>(&mut self, msg: Msg, ctx: &Ctx<'_>, api: &A) -> Out {
        match msg {
            Msg::OpenList(uri) => Update::event(Event::OpenList(uri)),
            Msg::OpenPack(uri) => Update::event(Event::OpenPack(uri)),
            Msg::OpenProfile(did) => Update::event(Event::OpenProfile(did)),
            Msg::OpenUrl(url) => Update::event(Event::OpenUrl(url)),
            Msg::OpenFeed(source, name) => Update::event(Event::OpenFeed(source, name)),
            Msg::Retry => Update::event(Event::Retry),
            Msg::SignInAgain => Update::event(Event::SignInAgain),
            Msg::Loaded(uri, result, more) => self.loaded(uri, result, more),
            Msg::More(uri) => {
                let Some(s) = self.screens.get_mut(&uri) else {
                    return Update::none();
                };
                let Some(cursor) = s.cursor.clone().filter(|_| !s.loading) else {
                    return Update::none();
                };
                s.loading = true;
                Update::task(fetch_list(api, uri, Some(cursor)))
            }
            Msg::Tab(uri, tab) => {
                self.screens.entry(uri).or_default().tab = tab;
                Update::event(Event::TabChanged)
            }
            Msg::Query(uri, q) => {
                let s = self.screens.entry(uri.clone()).or_default();
                s.query = q;
                s.seq += 1;
                if s.query.trim().is_empty() {
                    s.results.clear();
                    return Update::none();
                }
                let seq = s.seq;
                Update::task(Task::perform(
                    tokio::time::sleep(Duration::from_millis(250)),
                    move |()| Msg::Suggest(uri.clone(), seq),
                ))
            }
            Msg::Suggest(uri, seq) => {
                let Some(s) = self.screens.get(&uri).filter(|s| s.seq == seq) else {
                    return Update::none();
                };
                let q = s.query.trim().to_string();
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.typeahead(&q).await },
                    move |r| Msg::Results(uri.clone(), seq, r),
                ))
            }
            Msg::Results(uri, seq, Ok(found)) => {
                if let Some(s) = self.screens.get_mut(&uri).filter(|s| s.seq == seq) {
                    let urls: Vec<String> = found.iter().filter_map(|a| a.avatar.clone()).collect();
                    s.results = found;
                    return Update::event(Event::Images(urls));
                }
                Update::none()
            }
            Msg::Results(_, _, Err(e)) => Update::event(Event::ErrorShown(e)),
            Msg::Add(uri, who) => {
                let s = self.screens.entry(uri.clone()).or_default();
                if s.items.iter().any(|i| i.subject.did == who.did)
                    || !s.busy.insert(who.did.clone())
                {
                    return Update::none();
                }
                let api = api.clone();
                let (list, did) = (uri.clone(), who.did.clone());
                Update::task(Task::perform(
                    async move { api.add_to_list(&list, &did).await },
                    move |r| Msg::Added(uri.clone(), who.clone(), r),
                ))
            }
            Msg::Added(uri, who, result) => {
                let s = self.screens.entry(uri).or_default();
                s.busy.remove(&who.did);
                match result {
                    Ok(item) => {
                        s.items.insert(
                            0,
                            ListItem {
                                uri: item,
                                subject: who,
                            },
                        );
                        if let Some(Ok(l)) = &mut s.list {
                            l.list_item_count = l.list_item_count.map(|n| n + 1);
                        }
                        Update::none()
                    }
                    Err(e) => Update::event(Event::Error(e)),
                }
            }
            Msg::Remove(uri, item) => {
                let s = self.screens.entry(uri.clone()).or_default();
                if !s.busy.insert(item.subject.did.clone()) {
                    return Update::none();
                }
                let api = api.clone();
                let record = item.uri.clone();
                Update::task(Task::perform(
                    async move { api.delete(&record).await },
                    move |r| Msg::Removed(uri.clone(), item.clone(), r),
                ))
            }
            Msg::Removed(uri, item, result) => {
                let s = self.screens.entry(uri).or_default();
                s.busy.remove(&item.subject.did);
                match result {
                    Ok(()) => {
                        s.items.retain(|i| i.uri != item.uri);
                        if let Some(Ok(l)) = &mut s.list {
                            l.list_item_count = l.list_item_count.map(|n| n.saturating_sub(1));
                        }
                        Update::none()
                    }
                    Err(e) => Update::event(Event::Error(e)),
                }
            }
            Msg::Mute(uri, on) => {
                self.with_list(&uri, |l| l.viewer.muted = on);
                let api = api.clone();
                let u = uri.clone();
                Update::task(Task::perform(
                    async move { api.mute_list(&u, on).await },
                    move |r| Msg::Muted(uri.clone(), on, r),
                ))
            }
            Msg::Muted(_, _, Ok(())) => Update::event(Event::Toast(
                "Done. Muting a list takes effect in a moment.".into(),
            )),
            Msg::Muted(uri, on, Err(e)) => {
                self.with_list(&uri, |l| l.viewer.muted = !on);
                Update::event(Event::Error(e))
            }
            Msg::Block(uri, current) => {
                let api = api.clone();
                let u = uri.clone();
                Update::task(Task::perform(effects::block(api, u, current), move |r| {
                    Msg::Blocked(uri.clone(), r)
                }))
            }
            Msg::Blocked(uri, Ok(record)) => {
                self.with_list(&uri, |l| l.viewer.blocked = record);
                Update::none()
            }
            Msg::Blocked(_, Err(e)) => Update::event(Event::Error(e)),
            Msg::PackLoaded(uri, result) => match result {
                Ok(pack) => {
                    let mut urls: Vec<String> = pack.creator.avatar.iter().cloned().collect();
                    urls.extend(pack.feeds.iter().filter_map(|f| f.avatar.clone()));
                    let list = pack.list.as_ref().map(|l| l.uri.clone());
                    self.packs.insert(uri, Ok(*pack));
                    let task = list.map_or(Task::none(), |l| self.ensure_list(api, l));
                    Update::task(task).and(Event::Images(urls))
                }
                Err(e) => {
                    self.packs.insert(uri, Err(e.friendly()));
                    Update::event(Event::ErrorShown(e))
                }
            },
            Msg::ActorPacks(actor, result) => match result {
                Ok(packs) => {
                    self.actor_packs.insert(actor, Ok(packs));
                    Update::none()
                }
                Err(e) => {
                    self.actor_packs.insert(actor, Err(e.friendly()));
                    Update::event(Event::ErrorShown(e))
                }
            },
            Msg::FollowAll(uri) => {
                let Some(Ok(pack)) = self.packs.get(&uri) else {
                    return Update::none();
                };
                let Some(list) = pack.list.as_ref().map(|l| l.uri.clone()) else {
                    return Update::none();
                };
                if !self.following_all.insert(uri.clone()) {
                    return Update::none();
                }
                Update::task(Task::perform(
                    effects::follow_all(api.clone(), list, ctx.me.clone()),
                    move |r| Msg::FollowedAll(uri.clone(), r),
                ))
            }
            Msg::FollowedAll(uri, result) => {
                self.following_all.remove(&uri);
                match result {
                    Ok(done) => {
                        let follows: HashMap<Did, AtUri> = done.into_iter().collect();
                        for s in self.screens.values_mut() {
                            for i in &mut s.items {
                                if let Some(f) = follows.get(&i.subject.did) {
                                    i.subject.viewer.following = Some(Mine::Saved(f.clone()));
                                }
                            }
                        }
                        Update::event(Event::Toast(format!(
                            "Followed {} {}.",
                            follows.len(),
                            if follows.len() == 1 {
                                "person"
                            } else {
                                "people"
                            }
                        )))
                    }
                    Err(e) => Update::event(Event::Error(e)),
                }
            }
            Msg::PacksTab(on) => {
                self.packs_tab = on;
                self.load_lists_screen(api, ctx.me)
            }
            other => self.edit_update(other, ctx, api),
        }
    }

    fn loaded(&mut self, uri: AtUri, result: Result<Box<ListPage>, ApiError>, more: bool) -> Out {
        let s = self.screens.entry(uri).or_default();
        s.loading = false;
        match result {
            Ok(page) => {
                let ListPage { list, items } = *page;
                let mut urls: Vec<String> = list.avatar.iter().cloned().collect();
                urls.extend(items.items.iter().filter_map(|i| i.subject.avatar.clone()));
                if more {
                    s.items.extend(items.items);
                } else {
                    s.items = items.items;
                }
                s.cursor = items.cursor;
                // A moderation list has no posts to show.
                if list.is_mod() {
                    s.tab = ListTab::People;
                }
                s.list = Some(Ok(list));
                Update::event(Event::Images(urls))
            }
            Err(e) => {
                if !more {
                    s.list = Some(Err(e.friendly()));
                }
                Update::event(Event::ErrorShown(e))
            }
        }
    }

    /// Loads a list's details and first people, unless already loaded.
    pub fn ensure_list<A: ListsApi>(&mut self, api: &A, uri: AtUri) -> Task<Msg> {
        let s = self.screens.entry(uri.clone()).or_default();
        if s.loading || matches!(s.list, Some(Ok(_))) {
            return Task::none();
        }
        s.loading = true;
        fetch_list(api, uri, None)
    }

    /// Loads a starter pack, or its people if the pack is already here.
    pub fn load_pack<A: ListsApi>(&mut self, api: &A, uri: AtUri) -> Task<Msg> {
        if let Some(Ok(p)) = self.packs.get(&uri) {
            let list = p.list.as_ref().map(|l| l.uri.clone());
            return list.map_or(Task::none(), |l| self.ensure_list(api, l));
        }
        let api = api.clone();
        let u = uri.clone();
        Task::perform(async move { api.starter_pack(&u).await }, move |r| {
            Msg::PackLoaded(uri.clone(), r.map(Box::new))
        })
    }

    /// Loads someone's starter packs, unless already loaded.
    pub fn load_actor_packs<A: ListsApi>(&self, api: &A, actor: String) -> Task<Msg> {
        if self.actor_packs.contains_key(&actor) {
            return Task::none();
        }
        let api = api.clone();
        let a = actor.clone();
        Task::perform(async move { api.actor_starter_packs(&a).await }, move |r| {
            Msg::ActorPacks(actor.clone(), r)
        })
    }

    /// Your lists and starter packs, for the Lists screen.
    pub fn load_lists_screen<A: ListsApi>(&self, api: &A, me: &Did) -> Out {
        Update::task(self.load_actor_packs(api, me.to_string())).and(Event::NeedMyLists)
    }
}

fn fetch_list<A: ListsApi>(api: &A, uri: AtUri, cursor: Option<String>) -> Task<Msg> {
    let api = api.clone();
    let more = cursor.is_some();
    let u = uri.clone();
    Task::perform(async move { api.list(&u, cursor).await }, move |r| {
        Msg::Loaded(uri.clone(), r.map(Box::new), more)
    })
}
