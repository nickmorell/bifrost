//! Handling the chat feature's messages for starting chats, sending a post into a
//! chat, and who can message you.

use super::Out;
use super::message::{Ctx, Event, Msg};
use super::state::{Chats, MAX_MEMBERS, NewChat, Share, merge};
use super::update::toast;
use crate::api::ApiError;
use crate::api::service::ChatApi;
use crate::features::Update;
use crate::model::{AllowIncoming, Did};
use iced::Task;
use std::time::Duration;

/// The id of the New chat dialog's search field.
pub(super) const NEW_CHAT_SEARCH: &str = "new-chat-search";

impl Chats {
    pub(super) fn people_update<A: ChatApi>(&mut self, msg: Msg, ctx: &Ctx<'_>, api: &A) -> Out {
        match msg {
            Msg::MessagePerson(did) => {
                if self.opening {
                    return Update::none();
                }
                self.opening = true;
                Update::task(start(api, vec![ctx.me.clone(), did]))
            }
            Msg::Started(result) => {
                self.opening = false;
                if let Some(n) = &mut self.new_chat {
                    n.starting = false;
                }
                match result {
                    Ok(c) => {
                        let c = *c;
                        self.new_chat = None;
                        let id = c.id.clone();
                        let list = self.list(c.is_request());
                        if !list.ids.contains(&id) {
                            list.ids.insert(0, id.clone());
                        }
                        self.convos.insert(id.clone(), c);
                        self.update(Msg::Open(id), ctx, api)
                    }
                    Err(ApiError::ChatAccess) => {
                        self.no_access = true;
                        self.new_chat = None;
                        Update::event(Event::OpenChats)
                    }
                    Err(e) => {
                        let text = format!("Couldn't start the chat. {}", e.friendly());
                        match &mut self.new_chat {
                            Some(n) => {
                                n.error = Some(text);
                                Update::none()
                            }
                            None => toast(text),
                        }
                    }
                }
            }
            Msg::NewChat => {
                self.new_chat = Some(NewChat::default());
                Update::task(iced::widget::operation::focus(NEW_CHAT_SEARCH))
            }
            Msg::NewQuery(q) => {
                let Some(n) = &mut self.new_chat else {
                    return Update::none();
                };
                n.query = q;
                n.seq += 1;
                let seq = n.seq;
                Update::task(Task::perform(
                    tokio::time::sleep(Duration::from_millis(250)),
                    move |()| Msg::NewSuggest(seq),
                ))
            }
            Msg::NewSuggest(seq) => {
                let Some(n) = &self.new_chat else {
                    return Update::none();
                };
                let q = n.query.trim().to_string();
                if seq != n.seq || q.is_empty() {
                    return Update::none();
                }
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.typeahead(&q).await },
                    move |r| Msg::NewResults(seq, r),
                ))
            }
            Msg::NewResults(seq, result) => {
                let Some(n) = &mut self.new_chat else {
                    return Update::none();
                };
                if seq != n.seq {
                    return Update::none();
                }
                n.results = result.unwrap_or_default();
                n.results.retain(|a| a.did != *ctx.me);
                let urls: Vec<String> = n.results.iter().filter_map(|a| a.avatar.clone()).collect();
                Update::event(Event::Images(urls))
            }
            Msg::NewPick(a) => {
                if let Some(n) = &mut self.new_chat
                    && !n.picked.iter().any(|p| p.did == a.did)
                    && n.picked.len() < MAX_MEMBERS - 1
                {
                    n.picked.push(a);
                    n.query.clear();
                    n.results.clear();
                    n.error = None;
                }
                Update::task(iced::widget::operation::focus(NEW_CHAT_SEARCH))
            }
            Msg::NewUnpick(did) => {
                if let Some(n) = &mut self.new_chat {
                    n.picked.retain(|p| p.did != did);
                }
                Update::none()
            }
            Msg::NewStart => {
                let Some(n) = &mut self.new_chat else {
                    return Update::none();
                };
                if n.picked.is_empty() || n.starting {
                    return Update::none();
                }
                n.starting = true;
                n.error = None;
                let mut members: Vec<Did> = n.picked.iter().map(|a| a.did.clone()).collect();
                members.insert(0, ctx.me.clone());
                Update::task(start(api, members))
            }
            Msg::CloseNew => {
                self.new_chat = None;
                Update::none()
            }

            Msg::SharePost(post) => {
                self.share = Some(Share {
                    post,
                    text: String::new(),
                    sending: None,
                    sent: Vec::new(),
                    error: None,
                });
                let task = if self.accepted.loaded {
                    Task::none()
                } else {
                    self.load_list(api, false, false)
                };
                Update::task(task).and(Event::CloseMenu)
            }
            Msg::ShareText(t) => {
                if let Some(s) = &mut self.share {
                    s.text = t;
                }
                Update::none()
            }
            Msg::ShareTo(id) => {
                let Some(s) = &mut self.share else {
                    return Update::none();
                };
                if s.sending.is_some() || s.sent.contains(&id) {
                    return Update::none();
                }
                s.sending = Some(id.clone());
                s.error = None;
                let (text, post) = (s.text.trim().to_string(), s.post.clone());
                let api = api.clone();
                let i = id.clone();
                Update::task(Task::perform(
                    async move { api.send_message(&i, &text, Some(post)).await },
                    move |r| Msg::Shared(id.clone(), r.map(Box::new)),
                ))
            }
            Msg::Shared(id, result) => {
                if let Some(s) = &mut self.share {
                    s.sending = None;
                    match &result {
                        Ok(_) => s.sent.push(id.clone()),
                        Err(e) => s.error = Some(format!("Couldn't send it. {}", e.friendly())),
                    }
                }
                if let Ok(m) = result {
                    let m = *m;
                    if let Some(t) = self.threads.get_mut(&id) {
                        merge(&mut t.messages, vec![m.clone()]);
                    }
                    if let Some(mut c) = self.convos.get(&id).cloned() {
                        c.last_message = Some(m);
                        self.bump(c);
                    }
                }
                Update::none()
            }
            Msg::CloseShare => {
                self.share = None;
                Update::none()
            }

            Msg::AllowLoaded(result) => {
                // Without it the setting simply shows Bluesky's default.
                self.allow = Some(result.unwrap_or(AllowIncoming::Following));
                Update::none()
            }
            Msg::SetAllow(allow) => {
                let before = self.allow.replace(allow);
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.set_allow_incoming(allow).await },
                    move |r| Msg::AllowSaved(before.unwrap_or(AllowIncoming::Following), r),
                ))
            }
            Msg::AllowSaved(before, Err(e)) => {
                self.allow = Some(before);
                toast(format!("Couldn't save that. {}", e.friendly()))
            }
            // The rest are handled in `update`.
            _ => Update::none(),
        }
    }
}

/// Opens the chat with exactly these people, making it if there is none.
fn start<A: ChatApi>(api: &A, members: Vec<Did>) -> Task<Msg> {
    let api = api.clone();
    Task::perform(async move { api.convo_for(&members).await }, |r| {
        Msg::Started(r.map(Box::new))
    })
}
