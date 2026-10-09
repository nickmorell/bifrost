//! Handling the chat feature's messages for the chat list and an open chat.

use super::Out;
use super::message::{Ctx, Event, Msg};
use super::state::{Chats, Fetch, LOAD_OLDER_AT, MAX_MESSAGE, merge};
use crate::api::ApiError;
use crate::api::richtext::grapheme_len;
use crate::api::service::ChatApi;
use crate::features::Update;
use crate::model::{Convo, Page};
use iced::Task;
use std::time::Instant;

impl Chats {
    /// Handles one message. `api` is cloned into the tasks that need it.
    pub fn update<A: ChatApi>(&mut self, msg: Msg, ctx: &Ctx<'_>, api: &A) -> Out {
        if msg.expired() {
            return Update::event(Event::SessionExpired);
        }
        match msg {
            Msg::Open(id) => {
                self.menu = None;
                self.active_at = Some(Instant::now());
                if ctx.open == Some(id.as_str()) {
                    return Update::none();
                }
                Update::event(Event::OpenChat(id))
            }
            Msg::Tab(requests) => {
                self.showing_requests = requests;
                Update::task(self.load_list(api, requests, false))
            }
            Msg::Listed(requests, append, result) => self.listed(ctx, requests, append, result),
            Msg::MoreConvos => {
                let requests = self.showing_requests;
                let list = self.list(requests);
                if list.loading || list.cursor.is_none() {
                    return Update::none();
                }
                Update::task(self.load_list(api, requests, true))
            }
            Msg::Tick => {
                if self.no_access {
                    return Update::none();
                }
                Update::task(Task::batch([
                    self.load_list(api, false, false),
                    self.load_list(api, true, false),
                ]))
            }
            Msg::Poll => match ctx.open {
                Some(id) => {
                    let t = self.threads.entry(id.to_string()).or_default();
                    if t.polling || t.loading {
                        return Update::none();
                    }
                    t.polling = true;
                    Update::task(fetch(api, id.to_string(), Fetch::Poll, None))
                }
                None => Update::none(),
            },
            Msg::Messages(id, fetch, result) => self.messages(ctx, api, id, fetch, result),
            Msg::Scrolled(id, viewport) => {
                let from_top = viewport.absolute_offset_reversed().y;
                let t = self.threads.entry(id.clone()).or_default();
                if from_top < LOAD_OLDER_AT
                    && t.loaded
                    && !t.loading
                    && t.error.is_none()
                    && t.cursor.is_some()
                {
                    t.loading = true;
                    let cursor = t.cursor.clone();
                    return Update::task(fetch(api, id, Fetch::Older, cursor));
                }
                Update::none()
            }
            Msg::Draft(id, text) => {
                self.threads.entry(id).or_default().draft = text;
                Update::none()
            }
            Msg::Send(id) => {
                self.active_at = Some(Instant::now());
                let t = self.threads.entry(id.clone()).or_default();
                let text = t.draft.trim().to_string();
                if t.sending || text.is_empty() || grapheme_len(&text) > MAX_MESSAGE {
                    return Update::none();
                }
                t.sending = true;
                let api = api.clone();
                let i = id.clone();
                Update::task(Task::perform(
                    async move { api.send_message(&i, &text, None).await },
                    move |r| Msg::Sent(id.clone(), r.map(Box::new)),
                ))
            }
            Msg::Sent(id, result) => {
                let t = self.threads.entry(id.clone()).or_default();
                t.sending = false;
                match result {
                    Ok(m) => {
                        let m = *m;
                        t.draft.clear();
                        merge(&mut t.messages, vec![m.clone()]);
                        if let Some(mut c) = self.convos.get(&id).cloned() {
                            c.last_message = Some(m);
                            // Replying to a request accepts it.
                            if c.is_request() {
                                c.status = Some("accepted".into());
                            }
                            self.bump(c);
                        }
                        Update::none()
                    }
                    Err(e) => toast(format!("Couldn't send. {}", e.friendly())),
                }
            }
            Msg::Accept(id) => {
                let api = api.clone();
                let i = id.clone();
                Update::task(Task::perform(
                    async move { api.accept_convo(&i).await },
                    move |r| Msg::Accepted(id.clone(), r),
                ))
            }
            Msg::Accepted(id, Ok(())) => {
                if let Some(mut c) = self.convos.get(&id).cloned() {
                    c.status = Some("accepted".into());
                    self.bump(c);
                }
                Update::none()
            }
            Msg::Leave(id) => {
                if self.confirm_leave.as_deref() != Some(id.as_str()) {
                    self.confirm_leave = Some(id);
                    return Update::none();
                }
                self.confirm_leave = None;
                let api = api.clone();
                let i = id.clone();
                Update::task(Task::perform(
                    async move { api.leave_convo(&i).await },
                    move |r| Msg::Left(id.clone(), r),
                ))
            }
            Msg::Left(id, Ok(())) => {
                self.forget(&id);
                if ctx.open == Some(id.as_str()) {
                    return Update::event(Event::Back);
                }
                Update::none()
            }
            Msg::Mute(id, mute) => {
                if let Some(c) = self.convos.get_mut(&id) {
                    c.muted = mute;
                }
                self.count_unread();
                let api = api.clone();
                let i = id.clone();
                Update::task(Task::perform(
                    async move { api.mute_convo(&i, mute).await },
                    move |r| Msg::Muted(id.clone(), mute, r),
                ))
            }
            Msg::Muted(id, mute, Err(e)) => {
                if let Some(c) = self.convos.get_mut(&id) {
                    c.muted = !mute;
                }
                self.count_unread();
                toast(format!(
                    "Couldn't {} the chat. {}",
                    if mute { "mute" } else { "unmute" },
                    e.friendly()
                ))
            }
            Msg::Muted(_, _, Ok(())) | Msg::Read(Ok(())) | Msg::Reacted(.., Ok(())) => {
                Update::none()
            }
            Msg::Accepted(_, Err(e)) | Msg::Left(_, Err(e)) => toast(e.friendly()),
            // Read markers are retried the next time the chat is looked at.
            Msg::Read(Err(_)) => Update::none(),
            Msg::Menu(m) => {
                self.menu = m;
                Update::none()
            }
            Msg::Reacted(convo, message, value, add, Err(e)) => {
                self.set_reaction(ctx.me, &convo, &message, &value, !add);
                toast(format!("Couldn't save the reaction. {}", e.friendly()))
            }
            Msg::React(convo, message, value, add) => {
                self.menu = None;
                self.set_reaction(ctx.me, &convo, &message, &value, add);
                let api = api.clone();
                let (c, m, v) = (convo.clone(), message.clone(), value.clone());
                Update::task(Task::perform(
                    async move { api.react(&c, &m, &v, add).await },
                    move |r| Msg::Reacted(convo.clone(), message.clone(), value.clone(), add, r),
                ))
            }
            Msg::Delete(convo, message) => {
                self.menu = None;
                let api = api.clone();
                let (c, m) = (convo.clone(), message.clone());
                Update::task(Task::perform(
                    async move { api.delete_message(&c, &m).await },
                    move |r| Msg::Deleted(convo.clone(), message.clone(), r),
                ))
            }
            Msg::Deleted(convo, message, result) => match result {
                Ok(()) => {
                    if let Some(t) = self.threads.get_mut(&convo) {
                        t.messages.retain(|m| m.id != message);
                    }
                    Update::none()
                }
                Err(e) => toast(format!("Couldn't delete it. {}", e.friendly())),
            },
            Msg::OpenProfile(did) => Update::event(Event::OpenProfile(did)),
            Msg::CopyText(t) => Update::event(Event::CopyText(t)),
            Msg::Block(did) => Update::event(Event::Block(did)),
            Msg::Report(did) => Update::event(Event::Report(did)),
            Msg::SignOut => Update::event(Event::SignOut),
            Msg::OpenUrl(url) => Update::event(Event::OpenUrl(url)),
            Msg::Refresh => Update::event(Event::Refresh),
            Msg::Retry => Update::event(Event::Retry),
            Msg::SignInAgain => Update::event(Event::SignInAgain),
            Msg::ListScrolled(v) => Update::event(Event::ListScrolled(v)),
            other => self.people_update(other, ctx, api),
        }
    }

    fn listed(
        &mut self,
        ctx: &Ctx<'_>,
        requests: bool,
        append: bool,
        result: Result<Page<Convo>, ApiError>,
    ) -> Out {
        let list = self.list(requests);
        list.loading = false;
        match result {
            Ok(page) => {
                list.loaded = true;
                list.error = None;
                let ids: Vec<String> = page.items.iter().map(|c| c.id.clone()).collect();
                if append {
                    list.ids.extend(ids);
                    list.cursor = page.cursor;
                } else {
                    // A refresh puts the first page on top and keeps what was loaded
                    // below it.
                    let old = std::mem::take(&mut list.ids);
                    list.ids = ids;
                    if old.is_empty() || list.ids.len() < old.len() {
                        list.cursor = page.cursor;
                    }
                    for id in old {
                        if !list.ids.contains(&id) {
                            list.ids.push(id);
                        }
                    }
                }
                let mut urls = Vec::new();
                for c in page.items {
                    urls.extend(c.others(ctx.me).filter_map(|a| a.avatar.clone()).take(2));
                    // A chat that moved between the lists leaves the other one.
                    let other = self.list(!requests);
                    other.ids.retain(|i| *i != c.id);
                    self.convos.insert(c.id.clone(), c);
                }
                self.count_unread();
                Update::event(Event::Images(urls))
            }
            Err(ApiError::ChatAccess) => {
                self.no_access = true;
                Update::none()
            }
            Err(ApiError::SessionExpired) => Update::event(Event::SessionExpired),
            Err(e) => {
                list.error = Some(e.friendly());
                Update::none()
            }
        }
    }

    fn messages<A: ChatApi>(
        &mut self,
        ctx: &Ctx<'_>,
        api: &A,
        id: String,
        fetch: Fetch,
        result: Result<Page<crate::model::ChatMessage>, ApiError>,
    ) -> Out {
        let open = ctx.open == Some(id.as_str());
        let t = self.threads.entry(id.clone()).or_default();
        match fetch {
            Fetch::Poll => t.polling = false,
            _ => t.loading = false,
        }
        match result {
            Ok(page) => {
                t.error = None;
                let theirs = page.items.iter().any(|m| m.sender.did != *ctx.me);
                if fetch != Fetch::Poll || !t.loaded {
                    t.cursor = page.cursor;
                }
                t.loaded = true;
                let added = merge(&mut t.messages, page.items);
                if added && open {
                    self.active_at = Some(Instant::now());
                }
                if let Some(last) = t.messages.last().cloned()
                    && let Some(c) = self.convos.get_mut(&id)
                {
                    c.last_message = Some(last);
                }
                let unread = self.convos.get(&id).is_some_and(|c| c.unread_count > 0);
                let mut task = Task::none();
                if open && (fetch == Fetch::First || (added && theirs) || unread) {
                    task = self.mark_read(api, &id);
                }
                Update::task(task).and(Event::Images(self.avatars(&id)))
            }
            Err(ApiError::ChatAccess) => {
                self.no_access = true;
                Update::none()
            }
            Err(ApiError::SessionExpired) => Update::event(Event::SessionExpired),
            // A failed background check is retried on the next one.
            Err(_) if fetch == Fetch::Poll => Update::none(),
            Err(e) => {
                t.error = Some(e.friendly());
                Update::none()
            }
        }
    }
}

pub(super) fn toast(text: String) -> Out {
    Update::event(Event::Toast(text))
}

pub(super) fn fetch<A: ChatApi>(
    api: &A,
    id: String,
    fetch: Fetch,
    cursor: Option<String>,
) -> Task<Msg> {
    let api = api.clone();
    let i = id.clone();
    Task::perform(async move { api.messages(&i, cursor).await }, move |r| {
        Msg::Messages(id.clone(), fetch, r)
    })
}
