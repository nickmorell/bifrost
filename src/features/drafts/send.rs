//! Sending a scheduled draft: the posts it replies to or quotes are fetched again,
//! then it goes out as the composer would have sent it.

use super::Msg;
use crate::api::service::DraftsApi;
use crate::api::{ApiError, DraftImage, PostDraft, richtext};
use crate::model::{AtUri, Draft, Interaction, PostView, StrongRef};
use iced::Task;

/// A due draft with what the app keeps beside it.
#[derive(Debug, Clone)]
pub struct Sending {
    pub draft: Draft,
    /// Its images: bytes, alt text, width and height.
    pub images: Vec<(Vec<u8>, String, u32, u32)>,
    /// Your who-can-reply settings for new posts.
    pub interaction: Interaction,
}

/// Sends `s`, ending in [`Msg::Sent`].
pub(super) fn send(s: Sending, api: &impl DraftsApi) -> Task<Msg> {
    let api = api.clone();
    let id = s.draft.id;
    Task::perform(
        async move {
            let uris: Vec<AtUri> = s
                .draft
                .reply_to
                .iter()
                .chain(&s.draft.quote)
                .cloned()
                .collect();
            let posts = if uris.is_empty() {
                Vec::new()
            } else {
                api.posts(&uris).await?
            };
            let mut post = post_draft(s, &posts)?;
            if post.images.is_empty()
                && let Some(url) = richtext::first_link(&post.text)
            {
                // A post without its card is still worth sending.
                post.link_card = api.link_card(&url).await.ok();
            }
            api.create_post(post).await
        },
        move |r| Msg::Sent(id, r),
    )
}

/// The post to send for `s`, given the posts it replies to or quotes as they are now.
/// Fails when one of them is gone.
pub(super) fn post_draft(s: Sending, posts: &[PostView]) -> Result<PostDraft, ApiError> {
    let find = |uri: &AtUri, what: &str| {
        posts
            .iter()
            .find(|p| &p.uri == uri)
            .ok_or_else(|| ApiError::Other(format!("the post it {what} was deleted")))
    };
    let strong = |p: &PostView| StrongRef {
        uri: p.uri.clone(),
        cid: p.cid.clone(),
    };
    let reply = match &s.draft.reply_to {
        Some(uri) => {
            let parent = find(uri, "replies to")?;
            let root = parent
                .record
                .reply
                .as_ref()
                .map_or_else(|| strong(parent), |r| r.root.clone());
            Some((root, strong(parent)))
        }
        None => None,
    };
    let quote = match &s.draft.quote {
        Some(uri) => Some(strong(find(uri, "quotes")?)),
        None => None,
    };
    Ok(PostDraft {
        text: s.draft.text.trim_end().to_string(),
        reply,
        quote,
        images: s
            .images
            .into_iter()
            .map(|(bytes, alt, width, height)| DraftImage {
                bytes,
                alt,
                width,
                height,
            })
            .collect(),
        video: None,
        link_card: None,
        labels: Vec::new(),
        langs: if s.draft.lang.is_empty() {
            Vec::new()
        } else {
            vec![s.draft.lang]
        },
        interaction: s.interaction,
    })
}
