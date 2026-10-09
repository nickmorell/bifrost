//! Loading what the chat list, a chat and the setting need when they are shown.

use super::Out;
use super::message::{Ctx, Msg};
use super::state::{Chats, Fetch};
use super::update::fetch;
use crate::api::service::ChatApi;
use crate::features::Update;
use crate::model::Page;
use iced::Task;

impl Chats {
    /// The chat list is shown: refresh it, and the requests if they are showing.
    pub fn enter_list<A: ChatApi>(&mut self, api: &A) -> Task<Msg> {
        let requests = self.showing_requests;
        Task::batch([
            self.load_list(api, false, false),
            if requests {
                self.load_list(api, true, false)
            } else {
                Task::none()
            },
        ])
    }

    /// The chat `ctx.open` is shown: load it, or check it for new messages.
    pub fn enter_chat<A: ChatApi>(&mut self, ctx: &Ctx<'_>, api: &A) -> Out {
        let Some(id) = ctx.open.map(str::to_string) else {
            return Update::none();
        };
        self.menu = None;
        self.confirm_leave = None;
        let mut tasks = vec![iced::widget::operation::focus(input_id(&id))];
        if self.convos.get(&id).is_some_and(|c| c.unread_count > 0) {
            tasks.push(self.mark_read(api, &id));
        }
        let mut out = Update::none();
        let t = self.threads.entry(id.clone()).or_default();
        if t.loaded {
            out = self.update(Msg::Poll, ctx, api);
        } else if !t.loading {
            t.loading = true;
            tasks.push(fetch(api, id.clone(), Fetch::First, None));
        }
        if !self.convos.contains_key(&id) {
            let api = api.clone();
            tasks.push(Task::perform(async move { api.convo(&id).await }, |r| {
                Msg::Listed(
                    false,
                    true,
                    r.map(|c| Page {
                        items: vec![c],
                        cursor: None,
                    }),
                )
            }));
        }
        tasks.push(out.task);
        Update {
            task: Task::batch(tasks),
            events: out.events,
        }
    }

    /// Settings is shown: load who can message you, unless already loaded.
    pub fn enter_settings<A: ChatApi>(&self, api: &A) -> Task<Msg> {
        if self.allow.is_some() {
            return Task::none();
        }
        let api = api.clone();
        Task::perform(async move { api.allow_incoming().await }, Msg::AllowLoaded)
    }

    /// Refresh pressed on the chat list.
    pub fn refresh_list<A: ChatApi>(&mut self, api: &A) -> Task<Msg> {
        self.no_access = false;
        let requests = self.showing_requests;
        self.load_list(api, requests, false)
    }

    /// Refresh pressed in the chat `ctx.open`.
    pub fn refresh_chat<A: ChatApi>(&mut self, ctx: &Ctx<'_>, api: &A) -> Out {
        if let Some(t) = ctx.open.and_then(|id| self.threads.get_mut(id)) {
            t.error = None;
        }
        self.update(Msg::Poll, ctx, api)
    }

    pub(super) fn load_list<A: ChatApi>(
        &mut self,
        api: &A,
        requests: bool,
        append: bool,
    ) -> Task<Msg> {
        let list = self.list(requests);
        if list.loading || (append && list.cursor.is_none()) {
            return Task::none();
        }
        list.loading = true;
        let cursor = if append { list.cursor.clone() } else { None };
        let api = api.clone();
        Task::perform(
            async move { api.convos(requests, cursor).await },
            move |r| Msg::Listed(requests, append, r),
        )
    }

    pub(super) fn mark_read<A: ChatApi>(&mut self, api: &A, id: &str) -> Task<Msg> {
        if let Some(c) = self.convos.get_mut(id) {
            c.unread_count = 0;
        }
        self.count_unread();
        let api = api.clone();
        let id = id.to_string();
        Task::perform(async move { api.mark_convo_read(&id).await }, Msg::Read)
    }
}

/// The id of a chat's message field.
pub(super) fn input_id(convo: &str) -> String {
    format!("chat-input:{convo}")
}
