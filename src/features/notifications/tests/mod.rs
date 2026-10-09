//! The notifications feature against a fake Bluesky.

mod fake;

use super::update::page_with_posts;
use super::*;
use crate::api::ApiError;
use crate::components::Picture;
use crate::features::Pictures;
use crate::model::{AtUri, Notification, Page, PostView};
use fake::Fake;
use iced::Element;
use iced::widget::text;
use iced_test::simulator;
use serde_json::json;
use std::collections::HashSet;

const POST: &str = "at://did:plc:me/app.bsky.feed.post/p1";

fn note(reason: &str, who: &str, at: &str, following: bool) -> Notification {
    // Replies, mentions and quotes are posts themselves.
    let kind = match reason {
        "reply" | "mention" | "quote" => "post",
        other => other,
    };
    serde_json::from_value(json!({
        "uri": format!("at://{who}/app.bsky.feed.{kind}/{at}"), "cid": "c",
        "author": {
            "did": who,
            "handle": format!("{}.test", &who[8..]),
            "displayName": format!("{}", &who[8..]),
            "avatar": format!("https://a/{}.jpg", &who[8..]),
            "viewer": if following { json!({"following": "at://f"}) } else { json!({}) },
        },
        "reason": reason,
        "reasonSubject": (reason == "like").then_some(POST),
        "isRead": false, "indexedAt": at, "record": {}
    }))
    .unwrap()
}

fn post(uri: &str, text: &str) -> PostView {
    serde_json::from_value(json!({
        "uri": uri, "cid": "c",
        "author": {"did": "did:plc:me", "handle": "me.test"},
        "record": {"text": text, "createdAt": "2026-10-04T02:00:00Z"},
    }))
    .unwrap()
}

fn page(items: Vec<Notification>) -> Page<Notification> {
    Page {
        items,
        cursor: Some("c2".into()),
    }
}

fn run(n: &mut Notifications, msg: Msg, showing: bool) -> Vec<Event> {
    n.update(msg, &Ctx { showing }, &Fake::default()).events
}

fn loaded(n: &mut Notifications, append: bool, items: Vec<Notification>) -> Vec<Event> {
    let posts = vec![post(POST, "My post")];
    run(n, Msg::Loaded(append, Ok((page(items), posts))), false)
}

fn uris(n: &Notifications) -> Vec<String> {
    n.items.iter().map(|i| i.uri.to_string()).collect()
}

#[tokio::test]
async fn a_page_comes_with_the_posts_not_loaded_yet() {
    let reply = note("reply", "did:plc:ann", "2026-10-04T10:00:00Z", false);
    let fake = Fake {
        page: vec![
            note("like", "did:plc:bob", "2026-10-04T09:00:00Z", false),
            note("like", "did:plc:cat", "2026-10-04T08:00:00Z", false),
            reply.clone(),
            note("follow", "did:plc:dan", "2026-10-04T07:00:00Z", false),
        ],
        posts: vec![post(POST, "My post")],
        ..Fake::default()
    };
    let known = HashSet::from([AtUri::from(reply.uri.as_str())]);
    let (page, posts) = page_with_posts(fake.clone(), Some("c1".into()), known)
        .await
        .unwrap();
    assert_eq!(page.items.len(), 4);
    assert_eq!(posts.len(), 1);
    assert_eq!(
        fake.calls(),
        [
            "notifications Some(\"c1\")".to_string(),
            format!("posts {POST}"),
        ],
        "each post is asked for once, and known ones not at all"
    );
}

#[test]
fn pages_fill_the_list_and_refreshes_go_on_top() {
    let mut n = Notifications::default();
    assert!(n.is_stale());
    let a = note("like", "did:plc:ann", "2026-10-04T10:00:00Z", false);
    let b = note("follow", "did:plc:bob", "2026-10-04T09:00:00Z", false);
    let c = note("follow", "did:plc:cat", "2026-10-04T08:00:00Z", false);
    let events = loaded(&mut n, false, vec![a.clone()]);
    assert!(matches!(
        &events[..],
        [Event::Loaded { avatars, posts }] if avatars == &["https://a/ann.jpg"] && posts == &[POST]
    ));
    assert!(!n.is_stale() && n.has_more());
    assert_eq!(n.post(POST).unwrap().record.text, "My post");

    loaded(&mut n, true, vec![c.clone()]);
    assert_eq!(uris(&n), [a.uri.to_string(), c.uri.to_string()]);

    // A refresh brings b and a again: b goes on top, a is not doubled, c stays below.
    loaded(&mut n, false, vec![b.clone(), a.clone()]);
    assert_eq!(
        uris(&n),
        [b.uri.to_string(), a.uri.to_string(), c.uri.to_string()]
    );
}

#[test]
fn a_failed_load_is_reported_and_can_be_tried_again() {
    let mut n = Notifications::default();
    run(&mut n, Msg::Load { append: false }, false);
    assert!(!n.has_more(), "loading");
    let events = run(
        &mut n,
        Msg::Loaded(false, Err(ApiError::Other("down".into()))),
        false,
    );
    assert!(matches!(&events[..], [Event::Failed(_)]));
    assert!(n.is_stale() && !n.loading);
}

#[test]
fn new_arrivals_wait_for_the_list_unless_it_is_open() {
    let mut n = Notifications::default();
    let events = run(&mut n, Msg::Counted(Ok(3)), false);
    assert!(matches!(&events[..], [Event::Arrived]));
    assert_eq!(n.unread(), 3);
    assert!(
        run(&mut n, Msg::Counted(Ok(3)), false).is_empty(),
        "nothing new"
    );
    assert!(run(&mut n, Msg::Counted(Ok(2)), false).is_empty());
    assert_eq!(n.unread(), 2);

    // The list is open: the new ones are pulled in and seen at once.
    assert!(run(&mut n, Msg::Counted(Ok(5)), true).is_empty());
    assert_eq!(n.unread(), 0);
    assert!(n.loading);
    assert!(
        run(
            &mut n,
            Msg::Counted(Err(ApiError::Other("x".into()))),
            false
        )
        .is_empty()
    );
}

#[test]
fn opening_marks_everything_seen_and_reloads_only_when_stale() {
    let mut n = Notifications::default();
    loaded(&mut n, false, vec![]);
    n.loading = false;
    run(&mut n, Msg::Open { force: false }, false);
    assert!(!n.loading, "fresh and nothing unseen: nothing to load");

    run(&mut n, Msg::Counted(Ok(2)), false);
    run(&mut n, Msg::Open { force: false }, false);
    assert_eq!(n.unread(), 0);
    assert!(n.loading);

    let events = run(&mut n, Msg::Seen(Err(ApiError::SessionExpired)), false);
    assert!(matches!(
        &events[..],
        [Event::Quiet(ApiError::SessionExpired)]
    ));
}

#[test]
fn trimming_starts_over_but_keeps_the_filter_and_count() {
    let mut n = Notifications::default();
    let items = (0..5)
        .map(|i| {
            note(
                "follow",
                "did:plc:ann",
                &format!("2026-10-04T0{i}:00:00Z"),
                false,
            )
        })
        .collect();
    loaded(&mut n, false, items);
    run(&mut n, Msg::Filter(Filter::Mentions), false);
    run(&mut n, Msg::Counted(Ok(4)), false);
    n.trim(5);
    assert_eq!(n.items.len(), 5, "not over the limit");
    n.trim(4);
    assert!(n.items.is_empty() && n.is_stale());
    assert_eq!((n.filter, n.unread()), (Filter::Mentions, 4));
}

/// Draws posts as their text, with no pictures.
struct Plain;

impl Pictures for Plain {
    fn picture(&self, _: Option<&str>) -> Option<Picture> {
        None
    }
}

impl Host<Msg> for Plain {
    fn post<'a>(&'a self, post: &'a PostView) -> Element<'a, Msg> {
        text(post.record.text.as_str()).into()
    }
}

fn list(n: &Notifications) -> Element<'_, Msg> {
    View {
        notifications: n,
        host: &Plain,
        window: crate::components::Window {
            offset: 0.0,
            screen: 900.0,
        },
        lift: |m| m,
    }
    .body()
}

#[test]
fn the_list_groups_likes_and_offers_to_follow_back() {
    let mut n = Notifications::default();
    loaded(
        &mut n,
        false,
        vec![
            note("like", "did:plc:ann", "2026-10-04T10:00:00Z", false),
            note("like", "did:plc:bob", "2026-10-04T09:00:00Z", false),
            note("follow", "did:plc:cat", "2026-10-04T08:00:00Z", false),
            note("follow", "did:plc:dan", "2026-09-01T08:00:00Z", true),
        ],
    );
    let mut ui = simulator(list(&n));
    assert!(ui.find("and 1 other").is_ok());
    assert!(ui.find("My post").is_ok(), "the liked post's text");
    assert!(ui.find("bob").is_err(), "folded into the row");
    ui.click("and 1 other").unwrap();
    ui.click("Follow back").unwrap();
    let like = "at://did:plc:ann/app.bsky.feed.like/2026-10-04T10:00:00Z";
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Expand(u), Msg::FollowBack(d)] if u == like && d == "did:plc:cat"
    ));

    let events = run(&mut n, Msg::Expand(like.into()), false);
    assert!(events.is_empty());
    let mut ui = simulator(list(&n));
    assert!(
        ui.find("@bob.test").is_ok(),
        "everyone in the row is listed"
    );
    ui.click("@bob.test").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::OpenProfile(d)] if d == "did:plc:bob"
    ));
}

#[test]
fn mentions_show_the_post_and_say_when_there_are_none() {
    let mut n = Notifications::default();
    let reply = note("reply", "did:plc:ann", "2026-10-04T10:00:00Z", false);
    let posts = vec![post(reply.uri.as_str(), "Nice work!")];
    run(
        &mut n,
        Msg::Loaded(false, Ok((page(vec![reply]), posts))),
        false,
    );
    let mut ui = simulator(list(&n));
    assert!(ui.find("ann replied to you").is_ok());
    assert!(ui.find("Nice work!").is_ok());

    let mut empty = Notifications::default();
    loaded(
        &mut empty,
        false,
        vec![note("like", "did:plc:ann", "2026-10-04T10:00:00Z", false)],
    );
    let events = run(&mut empty, Msg::Filter(Filter::Mentions), false);
    assert!(matches!(&events[..], [Event::ScrollToTop]));
    let mut ui = simulator(list(&empty));
    assert!(ui.find("No replies, mentions or quotes yet.").is_ok());
}
