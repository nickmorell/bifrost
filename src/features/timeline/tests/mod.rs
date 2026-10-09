//! Timelines: loading pages, the Following filters, background checks, deleting and
//! undoing, trimming, and drawing only what is near the scroll position.

mod fake;
mod filter;
mod place;
mod view;

use super::merge::{Merge, merge_newest};
use super::*;
use crate::api::ApiError;
use crate::features;
use crate::model::{FeedItem, FeedSource, FollowingPrefs, MaybePost, Page, PostView};
use fake::Fake;
use serde_json::{Value, json};

const HOME: Key = Key::Feed(FeedSource::Following);

fn run(t: &mut Timelines, msg: Msg) -> features::Update<Msg, Event> {
    let prefs = FollowingPrefs {
        hide_reposts: true,
        ..FollowingPrefs::default()
    };
    let ctx = Ctx {
        following: &prefs,
        me: "did:plc:me",
        filters: &std::collections::BTreeMap::new(),
    };
    t.update(msg, &ctx, &Fake)
}

fn post_json(n: u32) -> Value {
    json!({
        "uri": format!("at://did:plc:a/app.bsky.feed.post/{n}"),
        "cid": "c",
        "author": {"did": "did:plc:a", "handle": "a.test"},
        "record": {"text": format!("post {n}"), "createdAt": "2026-01-01T00:00:00Z"},
    })
}

fn item(n: u32) -> FeedItem {
    serde_json::from_value(json!({ "post": post_json(n) })).unwrap()
}

fn repost(n: u32) -> FeedItem {
    serde_json::from_value(json!({
        "post": post_json(n),
        "reason": {
            "$type": "app.bsky.feed.defs#reasonRepost",
            "by": {"did": "did:plc:b", "handle": "b.test"},
        },
    }))
    .unwrap()
}

/// Post `n`, answering post `parent`.
fn reply(n: u32, parent: u32) -> FeedItem {
    let mut parent = post_json(parent);
    parent["$type"] = json!("app.bsky.feed.defs#postView");
    serde_json::from_value(json!({ "post": post_json(n), "reply": {"parent": parent} })).unwrap()
}

fn uris(items: &[FeedItem]) -> Vec<String> {
    items.iter().map(|i| i.post.uri.to_string()).collect()
}

fn shown(t: &Timelines, key: &Key) -> Vec<String> {
    uris(t.get(key).map_or(&[], Timeline::items))
}

fn page(items: Vec<FeedItem>, cursor: Option<&str>) -> Page<FeedItem> {
    Page {
        items,
        cursor: cursor.map(Into::into),
    }
}

/// A timeline loaded with `items`, more to come after `cursor`.
fn loaded(t: &mut Timelines, key: &Key, items: Vec<FeedItem>, cursor: Option<&str>) {
    let _ = run(
        t,
        Msg::Load {
            key: key.clone(),
            force: false,
        },
    );
    let _ = run(
        t,
        Msg::Loaded(key.clone(), Load::First, Ok(page(items, cursor))),
    );
}

#[test]
fn newest_page_adds_only_new_posts_on_top() {
    let shown = vec![item(3), item(2), item(1)];
    match merge_newest(&shown, vec![item(5), item(4), item(3), item(2)]) {
        Merge::Prepend(fresh) => assert_eq!(uris(&fresh), uris(&[item(5), item(4)])),
        Merge::Gap(_) => panic!("the pages overlap"),
    }
    match merge_newest(&shown, vec![item(3), item(2)]) {
        Merge::Prepend(fresh) => assert!(fresh.is_empty()),
        Merge::Gap(_) => panic!("nothing new"),
    }
    assert!(matches!(
        merge_newest(&shown, vec![item(9), item(8)]),
        Merge::Gap(_)
    ));
    assert!(matches!(
        merge_newest(&[], vec![item(1)]),
        Merge::Prepend(f) if f.len() == 1
    ));
}

#[test]
fn a_timeline_loads_once_unless_forced() {
    let mut t = Timelines::default();
    let key = Key::Bookmarks;
    let load = |force| Msg::Load {
        key: Key::Bookmarks,
        force,
    };
    assert_eq!(run(&mut t, load(false)).task.units(), 1);
    assert_eq!(run(&mut t, load(true)).task.units(), 0, "already loading");
    let out = run(
        &mut t,
        Msg::Loaded(
            key.clone(),
            Load::First,
            Ok(page(vec![item(2), item(1)], Some("c"))),
        ),
    );
    assert!(matches!(&out.events[..], [Event::Fetched(items)] if items.len() == 2));
    assert_eq!(run(&mut t, load(false)).task.units(), 0, "kept");
    assert_eq!(run(&mut t, load(true)).task.units(), 1);
    // A forced load of a shown timeline adds the newest on top.
    let _ = run(
        &mut t,
        Msg::Loaded(
            key.clone(),
            Load::Refresh,
            Ok(page(vec![item(3), item(2)], None)),
        ),
    );
    assert_eq!(shown(&t, &key), uris(&[item(3), item(2), item(1)]));
    // The cursor is the one the older pages left, so more can still load.
    assert_eq!(run(&mut t, Msg::More(key)).task.units(), 1);
}

#[test]
fn more_pages_skip_posts_already_shown_and_stop_at_the_end() {
    let mut t = Timelines::default();
    let key = Key::Author("did:plc:a".into(), crate::api::AuthorFilter::Posts);
    loaded(&mut t, &key, vec![item(3), item(2)], Some("c1"));
    assert_eq!(run(&mut t, Msg::More(key.clone())).task.units(), 1);
    assert_eq!(
        run(&mut t, Msg::More(key.clone())).task.units(),
        0,
        "loading"
    );
    let _ = run(
        &mut t,
        Msg::Loaded(
            key.clone(),
            Load::More,
            Ok(page(vec![item(2), item(1)], None)),
        ),
    );
    assert_eq!(shown(&t, &key), uris(&[item(3), item(2), item(1)]));
    assert_eq!(run(&mut t, Msg::More(key)).task.units(), 0, "no more pages");
}

#[test]
fn following_leaves_out_what_its_settings_hide_and_skips_empty_pages() {
    let mut t = Timelines::default();
    loaded(&mut t, &HOME, vec![item(3), repost(2)], Some("c1"));
    assert_eq!(shown(&t, &HOME), uris(&[item(3)]));
    let _ = run(&mut t, Msg::More(HOME));
    // Everything on the next page was left out, but there are more pages.
    let out = run(
        &mut t,
        Msg::Loaded(HOME, Load::More, Ok(page(vec![repost(1)], Some("c2")))),
    );
    assert!(matches!(&out.events[..], [Event::Fetched(i), Event::Empty(HOME)] if i.is_empty()));
    // Other timelines keep reposts.
    let mut t = Timelines::default();
    loaded(&mut t, &Key::Bookmarks, vec![repost(2)], None);
    assert_eq!(shown(&t, &Key::Bookmarks).len(), 1);
}

#[test]
fn background_checks_hold_new_posts_until_shown() {
    let mut t = Timelines::default();
    assert_eq!(
        run(&mut t, Msg::Peek(Key::Bookmarks)).task.units(),
        0,
        "not loaded"
    );
    let key = Key::Bookmarks;
    loaded(&mut t, &key, vec![item(2), item(1)], Some("c"));
    assert_eq!(run(&mut t, Msg::Peek(key.clone())).task.units(), 1);
    let out = run(
        &mut t,
        Msg::Loaded(
            key.clone(),
            Load::Peek,
            Ok(page(vec![item(3), item(2)], None)),
        ),
    );
    assert!(out.events.is_empty(), "pictures wait until shown");
    let tl = t.get(&key).unwrap();
    assert_eq!((tl.pending(), tl.pending_gap()), (1, false));
    assert_eq!(t.show_pending(&key), Some(0..1));
    assert_eq!(shown(&t, &key), uris(&[item(3), item(2), item(1)]));
    assert_eq!(t.show_pending(&key), None);

    // New posts that don't reach the shown ones replace them.
    let _ = run(&mut t, Msg::Peek(key.clone()));
    let _ = run(
        &mut t,
        Msg::Loaded(
            key.clone(),
            Load::Peek,
            Ok(page(vec![item(9), item(8)], Some("g"))),
        ),
    );
    assert!(t.get(&key).unwrap().pending_gap());
    assert_eq!(t.show_pending(&key), Some(0..2));
    assert_eq!(shown(&t, &key), uris(&[item(9), item(8)]));
}

#[test]
fn failures_show_in_the_timeline_but_background_ones_stay_quiet() {
    let mut t = Timelines::default();
    let key = Key::Bookmarks;
    loaded(&mut t, &key, vec![item(1)], Some("c"));
    let _ = run(&mut t, Msg::Peek(key.clone()));
    let quiet = run(
        &mut t,
        Msg::Loaded(key.clone(), Load::Peek, Err(ApiError::Other("x".into()))),
    );
    assert!(quiet.events.is_empty());
    assert!(!t.retry_more(&key), "nothing failed");
    let _ = run(&mut t, Msg::More(key.clone()));
    let out = run(
        &mut t,
        Msg::Loaded(key.clone(), Load::More, Err(ApiError::Other("x".into()))),
    );
    assert!(matches!(&out.events[..], [Event::Failed(_)]));
    assert!(t.retry_more(&key));
    assert!(!t.retry_more(&key), "forgotten once retried");
}

#[test]
fn deleted_posts_leave_every_timeline_and_come_back_on_undo() {
    let mut t = Timelines::default();
    loaded(&mut t, &HOME, vec![item(3), item(2), item(1)], None);
    loaded(&mut t, &Key::Bookmarks, vec![item(2)], None);
    let removed = t.remove_post(&item(2).post.uri);
    assert_eq!(removed.len(), 2);
    assert_eq!(shown(&t, &HOME), uris(&[item(3), item(1)]));
    assert!(shown(&t, &Key::Bookmarks).is_empty());
    t.restore(removed);
    assert_eq!(shown(&t, &HOME), uris(&[item(3), item(2), item(1)]));
    assert_eq!(shown(&t, &Key::Bookmarks), uris(&[item(2)]));
}

#[test]
fn a_deleted_post_is_no_longer_shown_above_its_replies() {
    let mut t = Timelines::default();
    loaded(&mut t, &HOME, vec![reply(3, 2), item(1)], None);
    let removed = t.remove_post(&item(2).post.uri);
    let above = |t: &Timelines| {
        t.get(&HOME).unwrap().items()[0]
            .reply
            .as_ref()
            .is_some_and(|r| matches!(r.parent, MaybePost::Post(_)))
    };
    assert!(!above(&t));
    assert_eq!(shown(&t, &HOME), uris(&[item(3), item(1)]));
    t.restore(removed);
    assert!(above(&t), "put back when Bluesky refuses");
}

#[test]
fn long_timelines_go_back_to_their_first_pages_unless_on_screen() {
    let mut t = Timelines::default();
    let key = Key::Bookmarks;
    loaded(&mut t, &key, vec![item(6), item(5)], Some("c1"));
    for (n, cursor) in [(4, "c2"), (2, "c3")] {
        let _ = run(&mut t, Msg::More(key.clone()));
        let _ = run(
            &mut t,
            Msg::Loaded(
                key.clone(),
                Load::More,
                Ok(page(vec![item(n), item(n - 1)], Some(cursor))),
            ),
        );
    }
    t.shorten(3, |_| true);
    assert_eq!(shown(&t, &key).len(), 6, "on screen");
    t.shorten(3, |_| false);
    assert_eq!(shown(&t, &key), uris(&[item(6), item(5)]));
    // Loading more carries on after the pages kept.
    assert_eq!(run(&mut t, Msg::More(key)).task.units(), 1);
}

#[test]
fn every_copy_of_a_post_can_be_changed_including_the_posts_answered() {
    let mut t = Timelines::default();
    loaded(&mut t, &HOME, vec![reply(2, 1)], None);
    let mut seen = Vec::new();
    t.each_post_mut(&mut |p: &mut PostView| seen.push(p.uri.to_string()));
    assert_eq!(seen, uris(&[item(2), item(1)]));
}
