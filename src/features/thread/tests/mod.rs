//! The thread feature against a fake Bluesky.

mod fake;

use super::*;
use crate::api::ApiError;
use crate::model::{AtUri, PostView, ThreadNode};
use fake::Fake;
use iced::Element;
use iced::widget::{Column, text};
use iced_test::simulator;
use serde_json::{Value, json};

const ME: &str = "did:plc:me";
const OP: &str = "did:plc:op";

fn post(did: &str, rkey: &str, likes: u64, following: bool) -> Value {
    json!({
        "uri": format!("at://{did}/app.bsky.feed.post/{rkey}"),
        "cid": "c",
        "author": {
            "did": did,
            "handle": format!("{}.test", &did[8..]),
            "viewer": if following { json!({"following": "at://f"}) } else { json!({}) },
        },
        "record": {"text": format!("post {rkey}"), "createdAt": "2026-10-04T02:00:00Z"},
        "likeCount": likes,
    })
}

fn node(post: Value, parent: Option<Value>, replies: Vec<Value>) -> Value {
    let mut n = json!({
        "$type": "app.bsky.feed.defs#threadViewPost",
        "post": post,
        "replies": replies,
    });
    if let Some(p) = parent {
        n["parent"] = p;
    }
    n
}

fn blocked() -> Value {
    json!({"$type": "app.bsky.feed.defs#blockedPost"})
}

fn gone() -> Value {
    json!({"$type": "app.bsky.feed.defs#notFoundPost"})
}

/// A reply chain `levels` deep, each reply answering the one before.
fn chain(levels: usize) -> Value {
    let mut n = node(post(OP, &format!("deep{levels}"), 0, false), None, vec![]);
    for l in (1..levels).rev() {
        n = node(post(OP, &format!("deep{l}"), 0, false), None, vec![n]);
    }
    n
}

/// A thread opened on `focus`, below the first post `root`, with these replies.
fn thread(replies: Vec<Value>) -> ThreadNode {
    let root = node(post(OP, "root", 0, false), Some(blocked()), vec![]);
    let focus = node(post("did:plc:bob", "focus", 0, false), Some(root), replies);
    serde_json::from_value(focus).unwrap()
}

const FOCUS: &str = "at://did:plc:bob/app.bsky.feed.post/focus";

fn loaded(threads: &mut Threads, uri: &str, node: ThreadNode) -> Vec<Event> {
    threads
        .update(
            Msg::Loaded(AtUri::from(uri), Ok(node)),
            ME,
            &Fake::default(),
        )
        .events
}

fn texts(posts: &[&PostView]) -> Vec<String> {
    posts.iter().map(|p| p.record.text.clone()).collect()
}

#[test]
fn loading_keeps_the_thread_and_asks_for_its_pictures() {
    let mut threads = Threads::default();
    let fake = Fake::default();
    let out = threads.update(Msg::Load(AtUri::from(FOCUS)), ME, &fake);
    assert!(out.events.is_empty());
    assert!(!threads.contains(FOCUS), "nothing is kept until it arrives");

    let events = loaded(&mut threads, FOCUS, thread(vec![chain(1), gone()]));
    assert!(matches!(&events[..], [Event::Loaded(u)] if u == FOCUS));
    assert!(threads.contains(FOCUS));
    assert_eq!(
        texts(&threads.posts_of(FOCUS)),
        ["post focus", "post root", "post deep1"]
    );
    assert_eq!(threads.posts().len(), 3);
    assert!(threads.posts_of("at://other").is_empty());
}

#[test]
fn failures_keep_their_wording_until_tried_again() {
    let mut threads = Threads::default();
    let events = threads
        .update(
            Msg::Loaded(AtUri::from(FOCUS), Err(ApiError::Other("down".into()))),
            ME,
            &Fake::default(),
        )
        .events;
    assert!(matches!(&events[..], [Event::Failed(ApiError::Other(_))]));
    assert!(threads.contains(FOCUS));
    threads.forget_failed(FOCUS);
    assert!(!threads.contains(FOCUS));

    loaded(&mut threads, FOCUS, thread(vec![]));
    threads.forget_failed(FOCUS);
    assert!(threads.contains(FOCUS), "a loaded thread is kept");
    threads.forget(FOCUS);
    assert!(!threads.contains(FOCUS));
}

#[test]
fn replies_read_op_then_you_then_people_you_follow_then_likes() {
    let mut replies: Vec<ThreadNode> = [
        blocked(),
        node(post("did:plc:ann", "liked", 50, false), None, vec![]),
        node(post("did:plc:bob", "followed", 0, true), None, vec![]),
        node(post("did:plc:cat", "more liked", 90, false), None, vec![]),
        node(post(ME, "mine", 0, false), None, vec![]),
        node(post(OP, "op", 0, false), None, vec![]),
    ]
    .into_iter()
    .map(|v| serde_json::from_value(v).unwrap())
    .collect();
    tree::sort_replies(&mut replies, OP, ME);
    let order: Vec<String> = replies
        .iter()
        .map(|n| match n {
            ThreadNode::Post(t) => t.post.record.text.clone(),
            _ => "unavailable".into(),
        })
        .collect();
    assert_eq!(
        order,
        [
            "post op",
            "post mine",
            "post followed",
            "post more liked",
            "post liked",
            "unavailable"
        ]
    );
}

#[test]
fn posts_above_end_at_the_first_unavailable_one() {
    let ThreadNode::Post(focus) = thread(vec![]) else {
        panic!("a post");
    };
    let above = tree::ancestors(&focus);
    assert_eq!(above.len(), 2);
    assert!(matches!(above[0], ThreadNode::Post(t) if t.post.record.text == "post root"));
    assert!(matches!(above[1], ThreadNode::Blocked));
}

#[test]
fn posts_can_be_found_and_changed_anywhere_in_a_thread() {
    let mut threads = Threads::default();
    loaded(&mut threads, FOCUS, thread(vec![chain(3)]));
    threads.each_post_mut(&mut |p| p.like_count += 1);
    let deep = threads
        .find_post(&format!("at://{OP}/app.bsky.feed.post/deep3"))
        .unwrap();
    assert_eq!(deep.like_count, 1);
    let root = threads
        .find_post(&format!("at://{OP}/app.bsky.feed.post/root"))
        .unwrap();
    assert_eq!(root.like_count, 1);
    assert!(threads.find_post("at://nowhere").is_none());
}

#[test]
fn deleted_posts_leave_threads_and_their_replies_move_up() {
    let mut threads = Threads::default();
    loaded(&mut threads, FOCUS, thread(vec![chain(3)]));
    let deep = |n: u32| format!("at://{OP}/app.bsky.feed.post/deep{n}");
    threads.remove_post(&deep(2));
    let posts = threads.posts_of(FOCUS);
    assert_eq!(
        texts(&posts),
        ["post focus", "post root", "post deep1", "post deep3"]
    );

    threads.remove_post(&format!("at://{OP}/app.bsky.feed.post/root"));
    assert_eq!(
        texts(&threads.posts_of(FOCUS)),
        ["post focus", "post deep1", "post deep3"]
    );

    threads.remove_post(FOCUS);
    assert!(
        !threads.contains(FOCUS),
        "a thread opened on it is forgotten"
    );
}

#[test]
fn trimming_keeps_wanted_threads_once_there_are_too_many() {
    let mut threads = Threads::default();
    for uri in ["at://a", "at://b", "at://c"] {
        loaded(&mut threads, uri, ThreadNode::NotFound);
    }
    threads.trim(3, |u| u == "at://b");
    assert!(threads.contains("at://a"), "not over the limit yet");
    threads.trim(2, |u| u == "at://b");
    assert!(!threads.contains("at://a") && threads.contains("at://b"));
    threads.clear();
    assert!(!threads.contains("at://b"));
}

/// Draws posts as their text and notes the reply rules, without an app.
struct Plain;

impl Host<Msg> for Plain {
    fn post<'a>(&'a self, post: &'a PostView, shape: Shape) -> Element<'a, Msg> {
        let mark = if shape.focused { "focused " } else { "" };
        text(format!("{mark}{}", post.record.text)).into()
    }
    fn reply_note(&self, root: &PostView) -> Option<String> {
        Some(format!("rules of {}", root.record.text))
    }
    fn retry<'a>(&self, why: String) -> Element<'a, Msg> {
        text(format!("retry: {why}")).into()
    }
    fn scroll<'a>(&self, content: Element<'a, Msg>) -> Element<'a, Msg> {
        Column::new().push(content).into()
    }
}

fn screen<'a>(threads: &'a Threads, uri: &str) -> Element<'a, Msg> {
    View {
        threads,
        host: &Plain,
        window: crate::components::Window {
            offset: 0.0,
            screen: 2000.0,
        },
        lift: |m| m,
    }
    .screen(uri)
}

#[test]
fn the_screen_shows_the_thread_and_offers_to_continue_deep_replies() {
    let mut threads = Threads::default();
    loaded(&mut threads, FOCUS, thread(vec![chain(5), blocked()]));
    let mut ui = simulator(screen(&threads, FOCUS));
    for shown in [
        "Blocked post",
        "post root",
        "focused post focus",
        "post deep4",
        "Blocked reply",
    ] {
        assert!(ui.find(shown).is_ok(), "{shown} is shown");
    }
    assert!(ui.find("post deep5").is_err(), "too deep to show");
    assert!(
        ui.find("rules of post root").is_err(),
        "the first post is blocked, so who can reply is unknown"
    );
    ui.click("Continue thread").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Open(u)] if u == &format!("at://{OP}/app.bsky.feed.post/deep4")
    ));
}

#[test]
fn the_screen_says_when_a_thread_is_loading_failed_or_gone() {
    let mut threads = Threads::default();
    let shows =
        |threads: &Threads, what: &str| simulator(screen(threads, FOCUS)).find(what).is_ok();
    assert!(shows(&threads, "Loading…"));

    let failed = Msg::Loaded(AtUri::from(FOCUS), Err(ApiError::Other("down".into())));
    assert_eq!(threads.update(failed, ME, &Fake::default()).events.len(), 1);
    assert!(shows(&threads, "retry: down"));

    loaded(&mut threads, FOCUS, ThreadNode::NotFound);
    assert!(shows(
        &threads,
        "This post is unavailable. It may have been deleted, or you may be blocked."
    ));

    let first: ThreadNode =
        serde_json::from_value(node(post(OP, "first", 0, false), None, vec![])).unwrap();
    loaded(&mut threads, FOCUS, first);
    assert!(shows(&threads, "rules of post first"));
}
