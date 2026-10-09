//! The composer against fakes for Bluesky and the file system.

mod fake;
mod thread;

use super::*;
use crate::api::{ApiError, DraftVideo};
use crate::components::Picture;
use crate::features::Pictures;
use crate::media::image::Prepared;
use crate::model::{AtUri, Author, Interaction, PostRecord, PostView};
use fake::Fake;
use iced::Element;
use iced::widget::text;
use iced::widget::text_editor::{Action, Edit};
use iced_test::simulator;
use serde_json::json;
use std::sync::Arc;

/// The app as far as the dialog needs it.
struct App {
    interaction: Interaction,
}

impl Pictures for App {
    fn picture(&self, _: Option<&str>) -> Option<Picture> {
        None
    }
}

impl Host<Msg> for App {
    fn rich_text<'a>(&'a self, record: &'a PostRecord) -> Element<'a, Msg> {
        text(&record.text).into()
    }
    fn my_avatar(&self) -> Option<&str> {
        None
    }
    fn tag_suggestions(&self, prefix: &str) -> Vec<String> {
        vec![format!("{prefix}lang")]
    }
    fn default_interaction(&self) -> &Interaction {
        &self.interaction
    }
    fn interaction_panel<'a>(
        &'a self,
        _: &Interaction,
        _: bool,
        _: &dyn Fn(Interaction) -> Msg,
    ) -> Element<'a, Msg> {
        text("Who can reply").into()
    }
}

fn app() -> App {
    App {
        interaction: Interaction::default(),
    }
}

fn dialog<'a>(c: &'a Composer, app: &'a App) -> Element<'a, Msg> {
    View {
        composer: c,
        host: app,
        lift: |m| m,
    }
    .dialog()
}

/// A post by did:plc:a, itself a reply when `root` is given.
fn post(uri: &str, root: Option<&str>) -> PostView {
    let mut record = json!({"$type": "app.bsky.feed.post", "text": "the post", "createdAt": "2026-01-01T00:00:00Z"});
    if let Some(root) = root {
        record["reply"] =
            json!({"root": {"uri": root, "cid": "r"}, "parent": {"uri": root, "cid": "r"}});
    }
    serde_json::from_value(json!({
        "uri": uri, "cid": "c", "author": {"did": "did:plc:a", "handle": "a.test"},
        "record": record, "indexedAt": "2026-01-01T00:00:00Z"
    }))
    .unwrap()
}

fn image() -> Prepared {
    Prepared {
        bytes: vec![1, 2, 3],
        width: 4,
        height: 3,
    }
}

fn video() -> DraftVideo {
    DraftVideo {
        bytes: vec![0; 10],
        name: "clip.mp4".into(),
        alt: String::new(),
        info: crate::api::VideoInfo::default(),
    }
}

/// Feeds a message through `update` and returns the events, in order.
fn events(c: &mut Composer, msg: Msg) -> Vec<Event> {
    c.update(msg, &Fake, &Fake).events
}

fn typed(c: &mut Composer, s: &str) {
    events(c, Msg::Edit(Action::Edit(Edit::Paste(Arc::new(s.into())))));
}

#[test]
fn mentions_are_suggested_and_picked() {
    let mut c = Composer::default();
    typed(&mut c, "hi @al");
    let alice: Author = serde_json::from_value(
        json!({"did": "did:plc:al", "handle": "alice.test", "avatar": "https://a/al"}),
    )
    .unwrap();
    let got = events(&mut c, Msg::Suggestions("al".into(), Ok(vec![alice])));
    assert!(matches!(&got[..], [Event::Images(u)] if u == &["https://a/al"]));
    // An answer for something no longer being typed is dropped.
    events(&mut c, Msg::Suggestions("x".into(), Ok(Vec::new())));
    assert_eq!(c.suggestions.len(), 1);
    events(&mut c, Msg::PickMention("alice.test".into()));
    assert_eq!(c.saved().text, "hi @alice.test ");
    assert!(c.suggestions.is_empty());
}

#[test]
fn hashtags_are_suggested_and_picked() {
    let mut c = Composer::default();
    typed(&mut c, "about #rus");
    assert_eq!(c.tag_partial.as_deref(), Some("rus"));
    let app = app();
    let mut ui = simulator(dialog(&c, &app));
    ui.click("#ruslang").unwrap();
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(matches!(&msgs[..], [Msg::PickTag(t)] if t == "ruslang"));
    events(&mut c, Msg::PickTag("rust".into()));
    assert_eq!(c.saved().text, "about #rust ");
}

#[test]
fn closing_asks_only_when_something_is_written() {
    let mut c = Composer::default();
    assert!(matches!(&events(&mut c, Msg::Cancel)[..], [Event::Close]));
    typed(&mut c, "draft");
    assert!(events(&mut c, Msg::Cancel).is_empty());
    assert!(c.is_confirming_close());
    let app = app();
    let mut ui = simulator(dialog(&c, &app));
    ui.click("Save draft").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::SaveDraft]
    ));
    assert!(matches!(
        &events(&mut c, Msg::SaveDraft)[..],
        [Event::SaveDraft]
    ));
    events(&mut c, Msg::KeepEditing);
    assert!(!c.is_confirming_close());
}

#[test]
fn a_reply_is_sent_under_the_threads_root() {
    let root = "at://did:plc:a/app.bsky.feed.post/root";
    let mut c = Composer::reply(post("at://did:plc:a/app.bsky.feed.post/2", Some(root)))
        .with_lang("fr".into());
    typed(&mut c, "  oui  ");
    let draft = c.draft();
    assert!(events(&mut c, Msg::Submit).is_empty());
    assert_eq!(draft.text, "  oui");
    let (r, parent) = draft.reply.as_ref().unwrap();
    assert_eq!(r.uri, root);
    assert_eq!(parent.uri, "at://did:plc:a/app.bsky.feed.post/2");
    assert_eq!(draft.langs, ["fr"]);
    assert!(c.is_posting());
    assert!(events(&mut c, Msg::Submit).is_empty(), "already posting");
    events(&mut c, Msg::Posted(Err(ApiError::Other("offline".into()))));
    assert!(!c.is_posting());
    assert_eq!(c.error.as_deref(), Some("Could not post: offline"));
}

#[test]
fn labels_only_go_with_media() {
    let mut c = Composer::default();
    typed(&mut c, "look");
    events(&mut c, Msg::ToggleLabel("nudity"));
    let draft = c.draft();
    assert!(c.can_post(&draft.text));
    assert!(draft.labels.is_empty());

    let mut c = Composer::default();
    events(&mut c, Msg::ImagesPicked(vec![Ok(image())]));
    events(&mut c, Msg::ToggleLabel("nudity"));
    events(&mut c, Msg::Alt(0, "a cat".into()));
    let draft = c.draft();
    assert!(c.can_post(&draft.text), "an image alone can be posted");
    assert_eq!(draft.labels, ["nudity"]);
    assert_eq!(draft.images[0].alt, "a cat");
}

#[test]
fn images_and_video_do_not_mix() {
    let mut c = Composer::default();
    let five =
        (0..5).map(|_| Ok(image())).chain([Err(
            crate::media::MediaError::Read("broken".into()).in_file("x.png")
        )]);
    events(&mut c, Msg::ImagesPicked(five.collect()));
    assert_eq!(c.images.len(), 4);
    assert_eq!(c.error.as_deref(), Some("x.png: broken"));
    events(&mut c, Msg::VideoPicked(Some(Ok(video()))));
    assert!(c.video.is_none(), "images are already attached");

    let mut c = Composer::default();
    events(&mut c, Msg::VideoPicked(Some(Ok(video()))));
    assert!(c.video.is_some());
    events(&mut c, Msg::ImagesPicked(vec![Ok(image())]));
    assert!(c.images.is_empty());
    assert_eq!(
        c.error.as_deref(),
        Some("A post can have one video, without images.")
    );
}

#[test]
fn drafts_round_trip() {
    let mut c =
        Composer::quote(post("at://did:plc:a/app.bsky.feed.post/q", None)).with_lang("de".into());
    typed(&mut c, "hallo");
    events(&mut c, Msg::ImagesPicked(vec![Ok(image())]));
    events(&mut c, Msg::Alt(0, "alt".into()));
    let saved = c.saved();
    assert_eq!(
        saved.quote.as_deref(),
        Some("at://did:plc:a/app.bsky.feed.post/q")
    );
    assert_eq!(saved.images, [(vec![1, 2, 3], "alt".to_string(), 4, 3)]);
    let back = Composer::reopen(saved.clone(), None, None);
    assert_eq!(
        back.saved(),
        Saved {
            quote: None,
            ..saved
        }
    );
}

#[test]
fn who_can_reply_asks_for_your_lists() {
    let mut c = Composer::default();
    assert!(matches!(
        &events(&mut c, Msg::ToggleInteraction)[..],
        [Event::NeedMyLists]
    ));
    let off = Interaction {
        quotes: false,
        ..Interaction::default()
    };
    events(&mut c, Msg::Interaction(off.clone()));
    let app = app();
    let mut ui = simulator(dialog(&c, &app));
    assert!(ui.find("Who can reply").is_ok());
    ui.click("Use these settings for new posts").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::KeepInteraction(i)] if *i == off
    ));
}

#[test]
fn post_is_offered_only_when_there_is_something_to_post() {
    let app = app();
    let c = Composer::default();
    let mut ui = simulator(dialog(&c, &app));
    assert!(ui.find("New post").is_ok());
    ui.click("Post").unwrap();
    assert_eq!(ui.into_messages().count(), 0);

    let mut c = Composer::default();
    typed(&mut c, "hello");
    let mut ui = simulator(dialog(&c, &app));
    ui.click("Post").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Submit]
    ));
}

#[test]
fn posting_offers_the_hashtags_and_reports_the_reply() {
    let parent = "at://did:plc:a/app.bsky.feed.post/2";
    let mut c = Composer::reply(post(parent, None));
    typed(&mut c, "hi #rust");
    let [Event::Tags(tags)] = &events(&mut c, Msg::Submit)[..] else {
        panic!("expected the hashtags");
    };
    assert_eq!(tags, &["rust"]);
    let sent = Msg::Posted(Ok(AtUri::from("at://did:plc:me/app.bsky.feed.post/3")));
    let [Event::Posted(Some(uri))] = &events(&mut c, sent)[..] else {
        panic!("expected the post it replies to");
    };
    assert_eq!(uri, parent);
}
