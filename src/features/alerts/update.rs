//! Finding what is new and wording it.

use super::message::{Ctx, Msg, Notice};
use super::notice::{MAX_EACH, for_message, for_notification, needs_text, summary};
use super::state::Alerts;
use crate::api::service::AlertsApi;
use crate::features::Update;
use crate::model::{AlertKind, AtUri, Notification, PostRecord};
use crate::moderation::muted_word;
use chrono::{DateTime, Utc};
use iced::Task;
use std::collections::HashMap;

type Out = Update<Msg, Notice>;

impl Alerts {
    pub fn update(&mut self, msg: Msg, ctx: &Ctx, api: &impl AlertsApi) -> Out {
        match msg {
            Msg::Seen(at) => {
                self.notifications = Some(at);
                self.messages = Some(at);
                Update::none()
            }
            Msg::Check => {
                let mut tasks = Vec::new();
                if self.notifications.is_some() && ctx.kinds.any_notifications() {
                    let api = api.clone();
                    tasks.push(Task::perform(
                        async move { api.notifications().await },
                        Msg::Notifications,
                    ));
                }
                if self.messages.is_some() && ctx.kinds.messages {
                    let api = api.clone();
                    tasks.push(Task::perform(
                        async move { api.convos().await },
                        Msg::Convos,
                    ));
                }
                Update::task(Task::batch(tasks))
            }
            Msg::Notifications(Ok(page)) => self.notifications(page.items, ctx, api),
            Msg::Posts(new, posts) => {
                let posts: HashMap<AtUri, PostRecord> = posts
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| (p.uri, p.record))
                    .collect();
                // A post with a muted word doesn't pop up at all, as it stays hidden
                // in the app.
                show(new.iter().filter_map(|n| {
                    let post = n.subject_uri().and_then(|u| posts.get(u));
                    if post.is_some_and(|p| muted_word(ctx.prefs, p).is_some()) {
                        return None;
                    }
                    Some(for_notification(n, post.map(|p| p.text.as_str())))
                }))
            }
            Msg::Convos(Ok(page)) => {
                let since = self.messages;
                let mut newest = since;
                let mut out = Vec::new();
                for c in &page.items {
                    let Some(m) = &c.last_message else { continue };
                    let Some(at) = Self::newer(since, &m.sent_at) else {
                        continue;
                    };
                    newest = newest.max(Some(at));
                    // Requests from people you haven't accepted stay quiet, like muted
                    // chats, and a message with a muted word never shows its text.
                    if !c.muted && !c.is_request() && c.unread_count > 0 && m.sender.did != *ctx.me
                    {
                        let from = c.members.iter().find(|a| a.did == m.sender.did);
                        let text =
                            ctx.kinds.message_text && muted_word(ctx.prefs, &m.body).is_none();
                        out.push(for_message(from, m, text));
                    }
                }
                self.messages = newest;
                if out.len() > MAX_EACH {
                    Update::event(summary(out.len(), "messages"))
                } else {
                    show(out.into_iter().rev())
                }
            }
            // A failed background check just waits for the next one.
            Msg::Notifications(Err(_)) | Msg::Convos(Err(_)) => Update::none(),
        }
    }

    /// Picks the new notifications of the kinds that are on, newest last.
    fn notifications(&mut self, items: Vec<Notification>, ctx: &Ctx, api: &impl AlertsApi) -> Out {
        let since = self.notifications;
        let mut newest: Option<DateTime<Utc>> = since;
        let mut new = Vec::new();
        for n in items {
            let Some(at) = Self::newer(since, &n.indexed_at) else {
                continue;
            };
            newest = newest.max(Some(at));
            if AlertKind::of_reason(&n.reason).is_some_and(|k| ctx.kinds.is_on(k)) {
                new.push(n);
            }
        }
        self.notifications = newest;
        new.reverse();
        if new.len() > MAX_EACH {
            return Update::event(summary(new.len(), "notifications"));
        }
        let uris: Vec<AtUri> = new
            .iter()
            .filter(|n| needs_text(n))
            .filter_map(|n| n.subject_uri().cloned())
            .collect();
        if uris.is_empty() {
            return show(new.iter().map(|n| for_notification(n, None)));
        }
        let api = api.clone();
        Update::task(Task::perform(
            async move { api.posts(&uris).await },
            move |posts| Msg::Posts(new, posts),
        ))
    }
}

fn show(notices: impl Iterator<Item = Notice>) -> Out {
    notices.fold(Update::none(), Update::and)
}
