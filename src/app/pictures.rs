//! Pictures: fetching them once, decoding them off the drawing thread, and which
//! pictures a post needs.

use super::{App, Message, images, media};
use crate::api;
use crate::api::ApiError;
use crate::components::Frames;
use crate::components::Picture;
use crate::integrations::Shown;
use crate::media::MediaError;
use crate::media::picture;
use crate::model::{Embed, EmbeddedRecord, FeedItem, FeedReplyContext, MaybePost, PostView};
use crate::storage::image_cache;
use iced::Task;
use iced::widget::image;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(String, Result<Vec<u8>, ApiError>),
    /// A picture finished decoding off the drawing thread.
    Decoded(String, Result<Ready, MediaError>),
    /// Only here to let the app fetch pictures that were drawn missing.
    Tick,
}

/// A decoded picture, ready to draw.
#[derive(Debug, Clone)]
pub enum Ready {
    Still(image::Handle),
    Animated(Frames),
}

/// Decodes a picture into pixels iced can draw as they are.
fn ready(bytes: &[u8]) -> Result<Ready, MediaError> {
    let handle = |p: picture::Pixels| image::Handle::from_rgba(p.width, p.height, p.rgba);
    Ok(match picture::decode(bytes)? {
        picture::Decoded::Still(p) => Ready::Still(handle(p)),
        picture::Decoded::Animated(frames) => Ready::Animated(Frames(Arc::new(
            frames.into_iter().map(|(p, d)| (handle(p), d)).collect(),
        ))),
    })
}

impl App {
    pub(super) fn pictures_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Loaded(url, Ok(bytes)) => Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || ready(&bytes))
                        .await
                        .map_err(|e| MediaError::Read(e.to_string()))
                        .and_then(|r| r)
                },
                move |r| Message::Pictures(Msg::Decoded(url.clone(), r)),
            ),
            Msg::Loaded(url, Err(_)) => {
                self.images.finish(url, None);
                Task::none()
            }
            Msg::Decoded(url, result) => {
                let state = match result {
                    Ok(Ready::Still(h)) => Some(images::State::Ready(h)),
                    Ok(Ready::Animated(frames)) => Some(images::State::Animated(frames)),
                    Err(_) => None,
                };
                self.images.finish(url, state);
                Task::none()
            }
            // Only here to let `update` fetch what was drawn missing.
            Msg::Tick => Task::none(),
        }
    }

    /// Whether loading placeholders pulse: not when motion is reduced.
    pub(crate) fn pulse(&self) -> bool {
        !self.settings.reduce_motion
    }

    /// A picture as far as it has loaded. A GIF stays on its first frame when motion
    /// is reduced or the data saver is on, unless it is open in the viewer.
    pub(crate) fn picture_at(&self, url: &str) -> Picture {
        match self.images.get(&images::sized(url)) {
            Some(images::State::Ready(h)) => Picture::Loaded(h.clone()),
            Some(images::State::Animated(frames))
                if (self.settings.reduce_motion || self.settings.data_saver)
                    && self.viewer.is_none() =>
            {
                frames
                    .first()
                    .map_or(Picture::Waiting { pulse: false }, |h| {
                        Picture::Loaded(h.clone())
                    })
            }
            Some(images::State::Animated(frames)) => Picture::Animated(frames.clone()),
            _ => Picture::Waiting {
                pulse: !self.image_failed(url) && self.pulse(),
            },
        }
    }

    /// A still of the picture, once it has loaded: a GIF's first frame.
    pub(crate) fn image(&self, url: &str) -> Option<&image::Handle> {
        match self.images.get(&images::sized(url)) {
            Some(images::State::Ready(h)) => Some(h),
            Some(images::State::Animated(frames)) => frames.first(),
            _ => None,
        }
    }

    /// Whether an image could not be fetched or decoded.
    pub(crate) fn image_failed(&self, url: &str) -> bool {
        self.images.failed(&images::sized(url))
    }

    /// Fetches pictures for what just arrived. With the data saver on, nothing is
    /// fetched until it is drawn.
    pub(super) fn request_images(
        &mut self,
        urls: impl IntoIterator<Item = String>,
    ) -> Task<Message> {
        if self.settings.data_saver {
            return Task::none();
        }
        self.fetch_images(urls)
    }

    /// Fetches these pictures, unless already loaded or on their way.
    pub(super) fn fetch_images(&mut self, urls: impl IntoIterator<Item = String>) -> Task<Message> {
        let mut tasks = Vec::new();
        for url in urls {
            let url = images::sized(&url).into_owned();
            if !self.images.start(&url) {
                continue;
            }
            let u = url.clone();
            tasks.push(Task::perform(
                async move {
                    if let Some(bytes) = image_cache::read(&u).await {
                        return Ok(bytes);
                    }
                    let bytes = api::fetch_bytes(&u).await?;
                    image_cache::write(&u, bytes.clone()).await;
                    Ok(bytes)
                },
                move |r| Message::Pictures(Msg::Loaded(url.clone(), r)),
            ));
        }
        Task::batch(tasks)
    }

    pub(super) fn integration_images(&self, post: &PostView, out: &mut Vec<String>) {
        if let Some(Embed::External { external }) = &post.embed {
            match self.integration_embed(&external.uri, Some(&external.title)) {
                Some(Shown::Card(card)) => out.extend(card.thumbnail_url),
                Some(Shown::Gif(url)) => out.push(url),
                None => {}
            }
        }
    }
}

pub(super) fn post_images(p: &PostView, out: &mut Vec<String>) {
    out.extend(p.author.avatar.clone());
    if let Some(e) = &p.embed {
        embed_images(e, out);
    }
}

pub(super) fn embed_images(e: &Embed, out: &mut Vec<String>) {
    match e {
        Embed::Images { images } => out.extend(images.iter().map(|i| i.thumb.clone())),
        Embed::Video { thumbnail, .. } => out.extend(thumbnail.clone()),
        Embed::External { external } if media::is_gif_url(&external.uri) => {
            out.push(external.uri.clone());
        }
        Embed::External { external } => {
            out.extend(external.thumb.clone());
            out.extend(external.source.as_ref().and_then(|s| s.icon.clone()));
            // Pictures from an integration's own service are fetched only if it is
            // on; see App::integration_images.
        }
        Embed::Record { record } => record_images(record, out),
        Embed::RecordWithMedia { record, media } => {
            embed_images(media, out);
            record_images(&record.record, out);
        }
        Embed::Other => {}
    }
}

pub(super) fn record_images(r: &EmbeddedRecord, out: &mut Vec<String>) {
    if let EmbeddedRecord::Post(q) = r {
        out.extend(q.author.avatar.clone());
        for e in &q.embeds {
            embed_images(e, out);
        }
    }
}

pub(super) fn feed_item_images(item: &FeedItem) -> Vec<String> {
    let mut out = Vec::new();
    post_images(&item.post, &mut out);
    if let Some(FeedReplyContext {
        parent: MaybePost::Post(p),
    }) = &item.reply
    {
        out.extend(p.author.avatar.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn author(name: &str) -> Value {
        json!({"did": format!("did:plc:{name}"), "handle": format!("{name}.test"),
               "avatar": format!("https://cdn.bsky.app/img/avatar/plain/{name}")})
    }

    fn post(name: &str, embed: Option<Value>) -> Value {
        let mut p = json!({
            "uri": format!("at://did:plc:{name}/app.bsky.feed.post/1"),
            "cid": "c",
            "author": author(name),
            "record": {"text": "", "createdAt": "2026-01-01T00:00:00Z"},
        });
        if let Some(e) = embed {
            p["embed"] = e;
        }
        p
    }

    fn images_of(p: &Value) -> Vec<String> {
        let mut out = Vec::new();
        post_images(&serde_json::from_value(p.clone()).unwrap(), &mut out);
        out
    }

    fn quote(name: &str, embeds: &[Value]) -> Value {
        json!({
            "$type": "app.bsky.embed.record#viewRecord",
            "uri": format!("at://did:plc:{name}/app.bsky.feed.post/2"),
            "cid": "c",
            "author": author(name),
            "value": {"text": "", "createdAt": "2026-01-01T00:00:00Z"},
            "embeds": embeds,
        })
    }

    fn pics() -> Value {
        json!({"$type": "app.bsky.embed.images#view", "images": [
            {"thumb": "t1", "fullsize": "f1"}, {"thumb": "t2", "fullsize": "f2"}]})
    }

    #[test]
    fn a_post_needs_its_avatar_and_the_thumbnails_not_the_full_pictures() {
        assert_eq!(
            images_of(&post("a", Some(pics()))),
            ["https://cdn.bsky.app/img/avatar/plain/a", "t1", "t2"]
        );
        let video =
            json!({"$type": "app.bsky.embed.video#view", "playlist": "p", "thumbnail": "vt"});
        assert_eq!(images_of(&post("a", Some(video)))[1..], ["vt"]);
    }

    #[test]
    fn a_link_needs_its_thumbnail_and_a_bluesky_gif_itself() {
        let link = |uri: &str| {
            json!({"$type": "app.bsky.embed.external#view",
                   "external": {"uri": uri, "thumb": "lt"}})
        };
        assert_eq!(
            images_of(&post("a", Some(link("https://example.com"))))[1..],
            ["lt"]
        );
        let gif = "https://cdn.bsky.app/img/x.gif";
        assert_eq!(images_of(&post("a", Some(link(gif))))[1..], [gif]);
        // Nothing from elsewhere is fetched just because a post links it.
        let other = images_of(&post("a", Some(link("https://media.tenor.com/x.gif"))));
        assert_eq!(other[1..], ["lt"]);
    }

    #[test]
    fn quotes_bring_their_author_and_pictures() {
        let q = json!({"$type": "app.bsky.embed.record#view", "record": quote("q", &[pics()])});
        assert_eq!(
            images_of(&post("a", Some(q)))[1..],
            ["https://cdn.bsky.app/img/avatar/plain/q", "t1", "t2"]
        );
        let both = json!({
            "$type": "app.bsky.embed.recordWithMedia#view",
            "record": {"record": quote("q", &[])},
            "media": pics(),
        });
        assert_eq!(
            images_of(&post("a", Some(both)))[1..],
            ["t1", "t2", "https://cdn.bsky.app/img/avatar/plain/q"]
        );
        let gone = json!({"$type": "app.bsky.embed.record#view",
                          "record": {"$type": "app.bsky.embed.record#viewBlocked"}});
        assert_eq!(images_of(&post("a", Some(gone))).len(), 1);
    }

    #[test]
    fn a_reply_in_a_feed_also_needs_the_avatar_of_who_it_answers() {
        let mut parent = post("p", None);
        parent["$type"] = json!("app.bsky.feed.defs#postView");
        let item: FeedItem = serde_json::from_value(json!({
            "post": post("a", None),
            "reply": {"parent": parent},
        }))
        .unwrap();
        assert_eq!(
            feed_item_images(&item),
            [
                "https://cdn.bsky.app/img/avatar/plain/a",
                "https://cdn.bsky.app/img/avatar/plain/p"
            ]
        );
    }

    #[test]
    fn pictures_decode_off_the_drawing_thread_or_fail_cleanly() {
        let mut png = Vec::new();
        ::image::RgbaImage::new(2, 1)
            .write_to(
                &mut std::io::Cursor::new(&mut png),
                ::image::ImageFormat::Png,
            )
            .unwrap();
        assert!(matches!(ready(&png), Ok(Ready::Still(_))));
        assert!(ready(b"not a picture").is_err());
    }
}
