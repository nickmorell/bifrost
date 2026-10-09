//! Changing who can reply to and quote a post you already made.

use super::message::Known;
use crate::api::ApiError;
use crate::api::service::ActionsApi;
use crate::features::Update;
use crate::model::{AtUri, Interaction, ThreadgateView};
use iced::Task;

/// A post's reply and quote settings being edited.
#[derive(Debug, Clone)]
pub struct Who {
    pub uri: AtUri,
    /// The first post of its thread: only then can replies be limited.
    pub is_root: bool,
    /// `None` while the current settings load.
    pub value: Option<Interaction>,
    pub saving: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Open(AtUri),
    Loaded(AtUri, Result<Interaction, ApiError>),
    Changed(Interaction),
    Save,
    Saved(Result<(), ApiError>),
    Close,
}

/// What the app has to do.
#[derive(Debug)]
pub enum Event {
    /// Your lists can be allowed to reply; fetch them.
    NeedMyLists,
    /// The post's thread now has these reply rules.
    Gate(AtUri, Option<ThreadgateView>),
    Toast(String),
    Failed(&'static str, ApiError),
}

type Out = Update<Msg, Event>;

/// Opens, edits and saves the settings in `editing`.
pub fn update(
    editing: &mut Option<Who>,
    msg: Msg,
    known: &impl Known,
    api: &impl ActionsApi,
) -> Out {
    match msg {
        Msg::Open(uri) => {
            let Some(post) = known.post(&uri) else {
                return Update::none();
            };
            *editing = Some(Who {
                uri: uri.clone(),
                is_root: post.record.reply.is_none(),
                value: None,
                saving: false,
            });
            let api = api.clone();
            Update::task(Task::perform(
                async move {
                    let r = api.post_interaction(&uri).await;
                    (uri, r)
                },
                |(uri, r)| Msg::Loaded(uri, r),
            ))
            .and(Event::NeedMyLists)
        }
        Msg::Loaded(uri, result) => {
            let Some(e) = editing.as_mut().filter(|e| e.uri == uri) else {
                return Update::none();
            };
            match result {
                Ok(v) => {
                    e.value = Some(v);
                    Update::none()
                }
                Err(err) => {
                    *editing = None;
                    Update::event(Event::Failed("load who can reply", err))
                }
            }
        }
        Msg::Changed(v) => {
            if let Some(e) = editing.as_mut().filter(|e| !e.saving) {
                e.value = Some(v);
            }
            Update::none()
        }
        Msg::Save => {
            let Some(e) = editing.as_mut().filter(|e| !e.saving) else {
                return Update::none();
            };
            let Some(v) = e.value.clone() else {
                return Update::none();
            };
            e.saving = true;
            let (api, uri, root) = (api.clone(), e.uri.clone(), e.is_root);
            Update::task(Task::perform(
                async move { api.set_post_interaction(&uri, v, root).await },
                Msg::Saved,
            ))
        }
        Msg::Saved(result) => {
            let Some(e) = editing.take() else {
                return Update::none();
            };
            match result {
                Ok(()) => {
                    let mut out = Update::event(Event::Toast("Saved who can interact.".into()));
                    if e.is_root {
                        let v = e.value.unwrap_or_default();
                        let gate = v.replies.as_ref().map(|r| ThreadgateView {
                            record: serde_json::json!({ "allow": r.to_json() }),
                            lists: Vec::new(),
                        });
                        out = Update::event(Event::Gate(e.uri, gate)).then(out);
                    }
                    out
                }
                Err(err) => Update::event(Event::Failed("save who can interact", err)),
            }
        }
        Msg::Close => {
            if !editing.as_ref().is_some_and(|e| e.saving) {
                *editing = None;
            }
            Update::none()
        }
    }
}
