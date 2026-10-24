//! Loading profiles with the feeds and lists people made, and the "Edit profile"
//! dialog.

use super::edit::{self, Editor, Outcome};
use super::message::{Ctx, Event, Msg};
use super::state::Profiles;
use super::tab::Tab;
use crate::api::ApiError;
use crate::api::service::ProfileApi;
use crate::features::Update;
use crate::media::files::Attachments;
use iced::Task;

type Out = Update<Msg, Event>;

impl Profiles {
    pub fn update(
        &mut self,
        msg: Msg,
        ctx: &Ctx,
        api: &impl ProfileApi,
        files: &impl Attachments,
    ) -> Out {
        match msg {
            Msg::Open(actor, tab) => {
                let profile = if self.by_actor.contains_key(&actor) {
                    Task::none()
                } else {
                    load_profile(actor.clone(), api)
                };
                let extra = match tab {
                    Tab::Feeds if !self.feeds.contains_key(&actor) => load_feeds(actor, api),
                    Tab::Lists if !self.lists.contains_key(&actor) => load_lists(actor, api),
                    _ => Task::none(),
                };
                Update::task(Task::batch([profile, extra]))
            }
            Msg::LoadLists(actor) => {
                if actor.is_empty() || matches!(self.lists.get(&actor), Some(Ok(_))) {
                    Update::none()
                } else {
                    Update::task(load_lists(actor, api))
                }
            }
            Msg::Loaded(actor, result) => match *result {
                Ok(p) => {
                    let icons = p.links.iter().filter_map(|l| l.icon.clone());
                    let urls = [p.avatar.clone(), p.banner.clone()].into_iter().flatten();
                    let urls = urls.chain(icons).collect();
                    self.by_actor.insert(actor, Ok(p));
                    Update::event(Event::Pictures(urls))
                }
                Err(e) => {
                    self.by_actor.insert(actor, Err(e.friendly()));
                    Update::event(Event::Failed(e))
                }
            },
            Msg::FeedsLoaded(actor, result) => match result {
                Ok(feeds) => {
                    let urls = feeds.iter().filter_map(|f| f.avatar.clone()).collect();
                    self.feeds.insert(actor, Ok(feeds));
                    Update::event(Event::Pictures(urls))
                }
                Err(e) => {
                    self.feeds.insert(actor, Err(e.friendly()));
                    Update::event(Event::Failed(e))
                }
            },
            Msg::ListsLoaded(actor, result) => match result {
                Ok(lists) => {
                    let urls = lists.iter().filter_map(|l| l.avatar.clone()).collect();
                    self.lists.insert(actor, Ok(lists));
                    Update::event(Event::Pictures(urls))
                }
                Err(e) => {
                    self.lists.insert(actor, Err(e.friendly()));
                    Update::event(Event::Failed(e))
                }
            },

            Msg::Tab(actor, tab) => Update::event(Event::Tab(actor, tab)),
            Msg::Follow(did, on) => Update::event(Event::Follow(did, on)),
            Msg::Mute(did, on) => Update::event(Event::Mute(did, on)),
            Msg::Block(did, on) => Update::event(Event::Block(did, on)),
            Msg::Message(did) => Update::event(Event::Message(did)),
            Msg::AddToLists(did, name) => Update::event(Event::AddToLists(did, name)),
            Msg::Report(did) => Update::event(Event::Report(did)),
            Msg::People(actor, kind) => Update::event(Event::People(actor, kind)),
            Msg::OpenFeed(source, name) => Update::event(Event::OpenFeed(source, name)),
            Msg::OpenUrl(url) => Update::event(Event::OpenUrl(url)),

            Msg::Edit => {
                if let Some(me) = ctx.me {
                    self.editor = Some(Editor::new(me));
                }
                Update::none()
            }
            Msg::Editor(edit::Msg::Saved(Err(ApiError::SessionExpired))) => {
                self.editor = None;
                Update::event(Event::Failed(ApiError::SessionExpired))
            }
            Msg::Editor(msg) => {
                let Some(editor) = &mut self.editor else {
                    return Update::none();
                };
                let (task, outcome) = editor.update(msg, api, files);
                let task = task.map(Msg::Editor);
                match outcome {
                    Outcome::None => Update::task(task),
                    Outcome::Close => {
                        self.editor = None;
                        Update::task(task)
                    }
                    Outcome::Saved => {
                        self.editor = None;
                        Update::task(task).and(Event::Saved)
                    }
                }
            }
        }
    }
}

fn load_profile(actor: String, api: &impl ProfileApi) -> Task<Msg> {
    let api = api.clone();
    let a = actor.clone();
    Task::perform(async move { api.profile(&a).await }, move |r| {
        Msg::Loaded(actor.clone(), Box::new(r))
    })
}

fn load_lists(actor: String, api: &impl ProfileApi) -> Task<Msg> {
    let api = api.clone();
    let a = actor.clone();
    Task::perform(async move { api.actor_lists(&a).await }, move |r| {
        Msg::ListsLoaded(actor.clone(), r)
    })
}

fn load_feeds(actor: String, api: &impl ProfileApi) -> Task<Msg> {
    let api = api.clone();
    let a = actor.clone();
    Task::perform(async move { api.actor_feeds(&a).await }, move |r| {
        Msg::FeedsLoaded(actor.clone(), r)
    })
}
