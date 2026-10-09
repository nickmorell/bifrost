//! The Drafts screen: what each draft says about itself, and its buttons.

use super::*;
use crate::model::Draft;
use iced_test::simulator;
use serde_json::json;

fn draft(id: u64, text: &str, extra: serde_json::Value) -> Draft {
    let mut v = json!({
        "id": id, "account": "did:plc:me", "saved_at": "2026-10-01T10:00:00Z", "text": text,
    });
    if let (Some(v), Some(extra)) = (v.as_object_mut(), extra.as_object()) {
        v.extend(extra.clone());
    }
    serde_json::from_value(v).unwrap()
}

#[test]
fn no_drafts_says_where_they_come_from() {
    let mut ui = simulator(View { drafts: &[] }.view());
    assert!(
        ui.find("No drafts. Anything you write and leave unsent is kept here.")
            .is_ok()
    );
}

#[test]
fn each_draft_shows_its_text_kind_and_images() {
    let image = json!({"file": "a.jpg", "alt": "", "width": 1, "height": 1});
    let drafts = [
        draft(
            1,
            "  ",
            json!({"reply_to": "at://did:plc:a/app.bsky.feed.post/1"}),
        ),
        draft(2, "Hello", json!({"images": [image.clone(), image]})),
    ];
    let mut ui = simulator(View { drafts: &drafts }.view());
    assert!(ui.find("(no text)").is_ok());
    assert!(ui.find("Hello").is_ok());
    assert!(ui.find(relative_line(&drafts[0], "Reply")).is_ok());
    assert!(ui.find(relative_line(&drafts[1], "2 images")).is_ok());
}

fn relative_line(d: &Draft, what: &str) -> String {
    format!("{} · {what}", crate::components::relative_time(&d.saved_at))
}

#[test]
fn the_buttons_open_and_delete() {
    let drafts = [draft(7, "Hi", json!({}))];
    let mut ui = simulator(View { drafts: &drafts }.view());
    ui.click("Open").unwrap();
    ui.click("Delete").unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Open(7), Msg::Delete(7)]
    ));
}

#[derive(Clone)]
struct Fake;

impl DraftsApi for Fake {
    async fn posts(&self, _: &[AtUri]) -> Result<Vec<PostView>, ApiError> {
        Ok(Vec::new())
    }
    async fn link_card(&self, _: &str) -> Result<crate::api::LinkCard, ApiError> {
        Err(ApiError::Other("offline".into()))
    }
    async fn create_post(&self, _: crate::api::PostDraft) -> Result<AtUri, ApiError> {
        Err(ApiError::Other("offline".into()))
    }
}

fn post(uri: &str) -> PostView {
    serde_json::from_value(json!({
        "uri": uri, "cid": "c", "indexedAt": "2026-10-01T10:00:00Z",
        "author": {"did": "did:plc:a", "handle": "a.test"},
        "record": {"text": "", "createdAt": "2026-10-01T10:00:00Z"},
    }))
    .unwrap()
}

#[test]
fn a_draft_without_a_reply_or_quote_opens_at_once() {
    let drafts = [draft(1, "Hi", json!({}))];
    let out = update(Msg::Open(1), &drafts, &Fake);
    let [Event::Open(o)] = &out.events[..] else {
        panic!("expected the draft to open");
    };
    assert!(o.draft.id == 1 && o.reply_to.is_none() && o.quote.is_none() && !o.reply_gone);
    assert!(update(Msg::Open(9), &drafts, &Fake).events.is_empty());
}

#[test]
fn a_draft_whose_reply_is_gone_opens_as_a_new_post() {
    let (reply, quote) = (
        "at://did:plc:a/app.bsky.feed.post/1",
        "at://did:plc:a/app.bsky.feed.post/2",
    );
    let drafts = [draft(1, "Hi", json!({"reply_to": reply, "quote": quote}))];
    let out = update(Msg::Fetched(1, Ok(vec![post(quote)])), &drafts, &Fake);
    let [Event::Open(o)] = &out.events[..] else {
        panic!("expected the draft to open");
    };
    assert!(o.reply_to.is_none() && o.reply_gone);
    assert_eq!(o.quote.as_ref().map(|q| q.uri.as_str()), Some(quote));

    let failed = Msg::Fetched(1, Err(ApiError::Other("offline".into())));
    assert!(matches!(
        update(failed, &drafts, &Fake).events[..],
        [Event::Failed(_)]
    ));
}

#[test]
fn scheduled_drafts_say_when_and_can_be_unscheduled() {
    let drafts = [draft(
        3,
        "Later",
        json!({"send_at": "2026-10-09T12:00:00Z"}),
    )];
    let mut ui = simulator(View { drafts: &drafts }.view());
    let when = crate::components::full_time("2026-10-09T12:00:00Z");
    assert!(
        ui.find(relative_line(&drafts[0], &format!("Scheduled for {when}")))
            .is_ok()
    );
    ui.click("Unschedule").unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Unschedule(3)]
    ));
    let plain = [draft(4, "Now", json!({}))];
    assert!(
        simulator(View { drafts: &plain }.view())
            .find("Unschedule")
            .is_err()
    );
}

#[test]
fn only_drafts_whose_time_has_come_are_due() {
    let drafts = [
        draft(1, "Past", json!({"send_at": "2026-10-08T09:00:00Z"})),
        draft(2, "Future", json!({"send_at": "2026-10-08T11:00:00Z"})),
        draft(3, "Unscheduled", json!({})),
    ];
    let now = "2026-10-08T10:00:00Z".parse().unwrap();
    let out = update(Msg::Due(now), &drafts, &Fake);
    assert!(matches!(&out.events[..], [Event::Due(ids)] if ids == &[1]));
    let early = "2026-10-08T08:00:00Z".parse().unwrap();
    assert!(update(Msg::Due(early), &drafts, &Fake).events.is_empty());

    let sent = update(
        Msg::Sent(1, Ok(AtUri::from("at://did:plc:me/app.bsky.feed.post/x"))),
        &drafts,
        &Fake,
    );
    assert!(matches!(sent.events[..], [Event::Sent(1)]));
    let failed = update(
        Msg::Sent(1, Err(ApiError::Other("no".into()))),
        &drafts,
        &Fake,
    );
    assert!(matches!(failed.events[..], [Event::SendFailed(1, _)]));
}

fn sending(extra: serde_json::Value) -> Sending {
    Sending {
        draft: draft(1, "Hello  ", extra),
        images: vec![(vec![1, 2], "A cat".into(), 4, 3)],
        interaction: crate::model::Interaction::default(),
    }
}

#[test]
fn a_scheduled_reply_goes_to_the_thread_it_was_written_for() {
    let parent = "at://did:plc:a/app.bsky.feed.post/2";
    let mut p = post(parent);
    p.record.reply = serde_json::from_value(json!({
        "root": {"uri": "at://did:plc:a/app.bsky.feed.post/1", "cid": "r"},
        "parent": {"uri": "at://did:plc:a/app.bsky.feed.post/1", "cid": "r"},
    }))
    .unwrap();
    let out = send::post_draft(sending(json!({"reply_to": parent, "lang": "en"})), &[p]).unwrap();
    assert_eq!(out.text, "Hello");
    assert_eq!(out.langs, ["en"]);
    assert_eq!(out.images[0].alt, "A cat");
    let (root, to) = out.reply.unwrap();
    assert_eq!(root.uri.as_str(), "at://did:plc:a/app.bsky.feed.post/1");
    assert_eq!(to.uri.as_str(), parent);

    let gone = send::post_draft(sending(json!({"quote": parent})), &[]);
    assert!(gone.unwrap_err().to_string().contains("quotes was deleted"));
}
