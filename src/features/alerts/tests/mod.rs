//! The alerts feature: what counts as new, which kinds pop up and how they read.

mod fake;

use super::*;
use crate::model::{
    Alerts as Kinds, Convo, Did, MutedWord, Notification, Page, PostView, Preferences,
};
use chrono::{DateTime, Utc};
use fake::Fake;
use serde_json::json;

const START: &str = "2026-10-07T12:00:00Z";
const ME: &str = "did:plc:me";

fn at(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn note(reason: &str, who: &str, when: &str) -> Notification {
    let kind = if matches!(reason, "reply" | "mention" | "quote") {
        "post"
    } else {
        reason
    };
    serde_json::from_value(json!({
        "uri": format!("at://{who}/app.bsky.feed.{kind}/{when}"), "cid": "c",
        "author": {"did": who, "handle": format!("{}.test", &who[8..]), "displayName": &who[8..]},
        "reason": reason, "indexedAt": when,
    }))
    .unwrap()
}

fn post(uri: &str, text: &str) -> PostView {
    serde_json::from_value(json!({
        "uri": uri, "cid": "c",
        "author": {"did": "did:plc:ann", "handle": "ann.test"},
        "record": {"text": text, "createdAt": START},
    }))
    .unwrap()
}

fn convo(id: &str, from: &str, when: &str, unread: u64) -> Convo {
    serde_json::from_value(json!({
        "id": id,
        "members": [
            {"did": ME, "handle": "me.test"},
            {"did": "did:plc:bob", "handle": "bob.test", "displayName": "Bob"},
        ],
        "lastMessage": {
            "$type": "chat.bsky.convo.defs#messageView",
            "id": format!("m-{id}"), "sender": {"did": from}, "sentAt": when, "text": "hi there",
        },
        "unreadCount": unread,
    }))
    .unwrap()
}

fn page<T>(items: Vec<T>) -> Page<T> {
    Page {
        items,
        cursor: None,
    }
}

/// Alerts that have seen everything up to `START`.
fn started() -> Alerts {
    let mut a = Alerts::default();
    let _ = run(&mut a, Msg::Seen(at(START)), Kinds::default());
    a
}

fn run(a: &mut Alerts, msg: Msg, kinds: Kinds) -> Vec<Notice> {
    run_with(a, msg, kinds, &Preferences::default())
}

fn run_with(a: &mut Alerts, msg: Msg, kinds: Kinds, prefs: &Preferences) -> Vec<Notice> {
    let me = Did::from(ME);
    a.update(
        msg,
        &Ctx {
            kinds: &kinds,
            me: &me,
            prefs,
        },
        &Fake,
    )
    .events
}

/// Kinds as they start, with message text shown.
fn with_text() -> Kinds {
    Kinds {
        message_text: true,
        ..Kinds::default()
    }
}

/// Preferences muting `word` everywhere.
fn muting(word: &str) -> Preferences {
    let mut p = Preferences::default();
    p.muted_words.push(MutedWord {
        value: word.into(),
        targets: vec!["content".into(), "tag".into()],
    });
    p
}

fn titles(notices: &[Notice]) -> Vec<&str> {
    notices.iter().map(|n| n.title.as_str()).collect()
}

#[test]
fn nothing_is_checked_before_the_app_says_where_to_start() {
    let mut a = Alerts::default();
    let me = Did::from(ME);
    let out = a.update(
        Msg::Check,
        &Ctx {
            kinds: &Kinds::default(),
            me: &me,
            prefs: &Preferences::default(),
        },
        &Fake,
    );
    assert_eq!(out.task.units(), 0);
    let mut a = started();
    let out = a.update(
        Msg::Check,
        &Ctx {
            kinds: &Kinds::default(),
            me: &me,
            prefs: &Preferences::default(),
        },
        &Fake,
    );
    assert_eq!(out.task.units(), 2);
}

#[test]
fn only_new_notifications_of_the_kinds_that_are_on_pop_up() {
    let mut a = started();
    let items = vec![
        note("follow", "did:plc:cat", "2026-10-07T12:03:00Z"),
        note("like", "did:plc:bob", "2026-10-07T12:02:00Z"),
        note("follow", "did:plc:dan", "2026-10-07T11:00:00Z"),
    ];
    let kinds = Kinds {
        follows: true,
        ..Kinds::default()
    };
    let shown = run(&mut a, Msg::Notifications(Ok(page(items.clone()))), kinds);
    assert_eq!(titles(&shown), ["cat followed you"]);
    // The same page again is not new any more.
    assert!(run(&mut a, Msg::Notifications(Ok(page(items))), kinds).is_empty());
}

#[test]
fn mentions_wait_for_their_post_and_carry_its_text() {
    let mut a = started();
    let mention = note("mention", "did:plc:ann", "2026-10-07T12:05:00Z");
    let out = a.update(
        Msg::Notifications(Ok(page(vec![mention.clone()]))),
        &Ctx {
            kinds: &Kinds::default(),
            me: &Did::from(ME),
            prefs: &Preferences::default(),
        },
        &Fake,
    );
    assert!(out.events.is_empty());
    assert_eq!(out.task.units(), 1);
    let text = post(mention.uri.as_str(), "hello @me");
    let shown = run(
        &mut a,
        Msg::Posts(vec![mention], Ok(vec![text])),
        Kinds::default(),
    );
    assert_eq!(
        shown,
        [Notice {
            title: "ann mentioned you".into(),
            body: "hello @me".into()
        }]
    );
}

#[test]
fn many_at_once_are_summed_up() {
    let mut a = started();
    let kinds = Kinds {
        likes: true,
        ..Kinds::default()
    };
    let items = (0..5)
        .map(|i| note("like", "did:plc:bob", &format!("2026-10-07T12:0{i}:30Z")))
        .collect();
    let shown = run(&mut a, Msg::Notifications(Ok(page(items))), kinds);
    assert_eq!(shown[0].body, "5 new notifications");
    assert_eq!(shown.len(), 1);
}

#[test]
fn new_messages_from_others_pop_up_once() {
    let mut a = started();
    let convos = vec![
        convo("c1", "did:plc:bob", "2026-10-07T12:01:00Z", 1),
        convo("c2", ME, "2026-10-07T12:02:00Z", 0),
        convo("c3", "did:plc:bob", "2026-10-07T11:00:00Z", 2),
    ];
    let shown = run(&mut a, Msg::Convos(Ok(page(convos.clone()))), with_text());
    assert_eq!(
        shown,
        [Notice {
            title: "Bob".into(),
            body: "hi there".into()
        }]
    );
    assert!(run(&mut a, Msg::Convos(Ok(page(convos))), with_text()).is_empty());
}

#[test]
fn messages_say_only_who_sent_them_unless_text_is_on() {
    let mut a = started();
    let convos = vec![convo("c1", "did:plc:bob", "2026-10-07T12:01:00Z", 1)];
    let shown = run(&mut a, Msg::Convos(Ok(page(convos))), Kinds::default());
    assert_eq!(
        shown,
        [Notice {
            title: "New message".into(),
            body: "From Bob".into()
        }]
    );
}

#[test]
fn message_requests_stay_quiet() {
    let mut a = started();
    let mut request = convo("c1", "did:plc:bob", "2026-10-07T12:01:00Z", 1);
    request.status = Some("request".into());
    assert!(run(&mut a, Msg::Convos(Ok(page(vec![request]))), with_text()).is_empty());
}

#[test]
fn muted_words_keep_text_out_of_notices() {
    let mut a = started();
    let convos = vec![convo("c1", "did:plc:bob", "2026-10-07T12:01:00Z", 1)];
    let shown = run_with(
        &mut a,
        Msg::Convos(Ok(page(convos))),
        with_text(),
        &muting("there"),
    );
    assert_eq!(titles(&shown), ["New message"]);
    assert!(!shown[0].body.contains("hi"));

    let mention = note("mention", "did:plc:ann", "2026-10-07T12:05:00Z");
    let text = post(mention.uri.as_str(), "spoilers for the finale");
    let shown = run_with(
        &mut a,
        Msg::Posts(vec![mention], Ok(vec![text])),
        Kinds::default(),
        &muting("spoilers"),
    );
    assert!(shown.is_empty(), "a muted post doesn't pop up");
}

#[test]
fn seeing_the_window_moves_the_start() {
    let mut a = started();
    let _ = run(
        &mut a,
        Msg::Seen(at("2026-10-07T13:00:00Z")),
        Kinds::default(),
    );
    let kinds = Kinds {
        follows: true,
        ..Kinds::default()
    };
    let items = vec![note("follow", "did:plc:cat", "2026-10-07T12:30:00Z")];
    assert!(run(&mut a, Msg::Notifications(Ok(page(items))), kinds).is_empty());
}

#[test]
fn the_check_says_what_it_fetches() {
    let mut alerts = Alerts::default();
    let kinds = Kinds::default();
    assert_eq!(
        alerts.fetches(&kinds),
        (false, false),
        "nothing before a start"
    );
    let _ = run(&mut alerts, Msg::Seen(at(START)), Kinds::default());
    assert_eq!(
        alerts.fetches(&kinds),
        (kinds.any_notifications(), kinds.messages)
    );
}
