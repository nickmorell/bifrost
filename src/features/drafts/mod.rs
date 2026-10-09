//! The Drafts screen: posts you started and left unsent, newest first, and opening
//! one again.
//!
//! The drafts themselves are kept on this computer by the app, which hands the list
//! to [`View`] and to [`update`]. Opening a draft fetches the posts it replies to or
//! quotes again, then asks the app to open the composer with [`Event::Open`].
//!
//! A draft can carry a time to be sent. On each [`Msg::Due`] the drafts whose time
//! has come are named in [`Event::Due`]; the app hands each back as [`Msg::Send`]
//! with its images, and hears how it went through [`Event::Sent`] or
//! [`Event::SendFailed`].

mod send;
mod view;

#[cfg(test)]
mod tests;

pub use send::Sending;
pub use view::View;

use crate::api::ApiError;
use crate::api::service::DraftsApi;
use crate::features::Update;
use crate::model::{AtUri, Draft, PostView};
use chrono::{DateTime, Utc};
use iced::Task;

#[derive(Debug, Clone)]
pub enum Msg {
    /// Open the draft with this id in the composer.
    Open(u64),
    Delete(u64),
    /// The posts a draft replies to or quotes, fetched again.
    Fetched(u64, Result<Vec<PostView>, ApiError>),
    /// Check for scheduled drafts whose time has come.
    Due(DateTime<Utc>),
    Send(Box<Sending>),
    Sent(u64, Result<AtUri, ApiError>),
    /// Keep the draft, but don't send it.
    Unschedule(u64),
}

#[derive(Debug)]
pub enum Event {
    Open(Box<Opening>),
    Delete(u64),
    Failed(ApiError),
    /// These scheduled drafts are due: send them.
    Due(Vec<u64>),
    /// The scheduled draft went out.
    Sent(u64),
    /// The scheduled draft couldn't be sent; it stays in Drafts.
    SendFailed(u64, ApiError),
    Unschedule(u64),
}

/// A draft to open in the composer, with the posts it replies to and quotes as they
/// are now.
#[derive(Debug)]
pub struct Opening {
    pub draft: Draft,
    pub reply_to: Option<PostView>,
    pub quote: Option<PostView>,
    /// The post it replies to was deleted, so it opens as a new post.
    pub reply_gone: bool,
}

pub fn update(msg: Msg, drafts: &[Draft], api: &impl DraftsApi) -> Update<Msg, Event> {
    let find = |id| drafts.iter().find(|d| d.id == id);
    match msg {
        Msg::Open(id) => {
            let Some(draft) = find(id) else {
                return Update::none();
            };
            let uris: Vec<AtUri> = draft.reply_to.iter().chain(&draft.quote).cloned().collect();
            if uris.is_empty() {
                return update(Msg::Fetched(id, Ok(Vec::new())), drafts, api);
            }
            let api = api.clone();
            Update::task(Task::perform(
                async move { api.posts(&uris).await },
                move |r| Msg::Fetched(id, r),
            ))
        }
        Msg::Delete(id) => Update::event(Event::Delete(id)),
        Msg::Due(now) => {
            let due: Vec<u64> = drafts
                .iter()
                .filter(|d| d.due(&now))
                .map(|d| d.id)
                .collect();
            if due.is_empty() {
                Update::none()
            } else {
                Update::event(Event::Due(due))
            }
        }
        Msg::Send(s) => Update::task(send::send(*s, api)),
        Msg::Sent(id, Ok(_)) => Update::event(Event::Sent(id)),
        Msg::Sent(id, Err(e)) => Update::event(Event::SendFailed(id, e)),
        Msg::Unschedule(id) => Update::event(Event::Unschedule(id)),
        Msg::Fetched(_, Err(e)) => Update::event(Event::Failed(e)),
        Msg::Fetched(id, Ok(posts)) => {
            let Some(draft) = find(id) else {
                return Update::none();
            };
            let find = |uri: &Option<AtUri>| {
                uri.as_ref()
                    .and_then(|u| posts.iter().find(|p| &p.uri == u).cloned())
            };
            let (reply_to, quote) = (find(&draft.reply_to), find(&draft.quote));
            Update::event(Event::Open(Box::new(Opening {
                reply_gone: draft.reply_to.is_some() && reply_to.is_none(),
                draft: draft.clone(),
                reply_to,
                quote,
            })))
        }
    }
}
