//! Lists of accounts against a fake Bluesky: loading, more pages, failures, and what
//! the rows ask for.

mod fake;

use super::*;
use crate::api::ApiError;
use crate::components::Picture;
use crate::features::Pictures;
use crate::model::{Author, Page, PeopleKind};
use fake::Fake;
use iced_test::simulator;
use serde_json::json;

const ME: &str = "did:plc:me";

fn author(name: &str, viewer: serde_json::Value) -> Author {
    serde_json::from_value(json!({
        "did": format!("did:plc:{name}"),
        "handle": format!("{name}.test"),
        "displayName": name,
        "avatar": format!("https://a/{name}.jpg"),
        "viewer": viewer,
    }))
    .unwrap()
}

fn page(items: Vec<Author>, cursor: Option<&str>) -> Page<Author> {
    Page {
        items,
        cursor: cursor.map(Into::into),
    }
}

fn loaded(
    p: &mut People,
    kind: PeopleKind,
    append: bool,
    r: Result<Page<Author>, ApiError>,
) -> Vec<Event> {
    p.update(Msg::Loaded(ME.into(), kind, append, r), &Fake)
        .events
}

struct NoPictures;

impl Pictures for NoPictures {
    fn picture(&self, _: Option<&str>) -> Option<Picture> {
        None
    }
}

#[test]
fn opening_loads_once_and_more_follows_the_cursor() {
    let api = Fake;
    let mut p = People::default();
    let open = || Msg::Open(ME.into(), PeopleKind::Followers);
    assert_eq!(p.update(open(), &api).task.units(), 1);
    assert_eq!(p.update(open(), &api).task.units(), 0, "already loading");
    let events = loaded(
        &mut p,
        PeopleKind::Followers,
        false,
        Ok(page(vec![author("ann", json!({}))], Some("c2"))),
    );
    assert!(matches!(&events[..], [Event::Pictures(urls)] if urls == &["https://a/ann.jpg"]));
    assert_eq!(p.update(open(), &api).task.units(), 0, "loaded");
    assert_eq!(
        p.update(Msg::More(ME.into(), PeopleKind::Followers), &api)
            .task
            .units(),
        1
    );
    let _ = loaded(
        &mut p,
        PeopleKind::Followers,
        true,
        Ok(page(vec![author("bob", json!({}))], None)),
    );
    assert_eq!(p.authors_mut().count(), 2);
    assert_eq!(
        p.update(Msg::More(ME.into(), PeopleKind::Followers), &api)
            .task
            .units(),
        0,
        "no more pages"
    );
}

#[test]
fn refresh_starts_over() {
    let mut p = People::default();
    let _ = loaded(
        &mut p,
        PeopleKind::Following,
        false,
        Ok(page(vec![author("ann", json!({}))], None)),
    );
    let api = Fake;
    assert_eq!(
        p.update(Msg::Refresh(ME.into(), PeopleKind::Following), &api)
            .task
            .units(),
        1
    );
    let _ = loaded(
        &mut p,
        PeopleKind::Following,
        false,
        Ok(page(vec![author("cat", json!({}))], None)),
    );
    let names: Vec<String> = p.authors_mut().map(|a| a.handle.clone()).collect();
    assert_eq!(names, ["cat.test"]);
}

#[test]
fn a_failed_page_shows_why_and_tries_the_same_page_again() {
    let api = Fake;
    let mut p = People::default();
    let _ = loaded(
        &mut p,
        PeopleKind::Followers,
        false,
        Ok(page(vec![author("ann", json!({}))], Some("c2"))),
    );
    let _ = p.update(Msg::More(ME.into(), PeopleKind::Followers), &api);
    let events = loaded(
        &mut p,
        PeopleKind::Followers,
        true,
        Err(ApiError::Other("offline".into())),
    );
    assert!(matches!(&events[..], [Event::Failed(_)]));
    let ui = simulator(
        View {
            people: &p,
            actor: ME,
            kind: PeopleKind::Followers,
            pictures: &NoPictures,
            badges: true,
        }
        .view(),
    );
    let mut ui = ui;
    ui.click("Try again").unwrap();
    let Some(Msg::Retry(actor, kind)) = ui.into_messages().next() else {
        panic!("expected a retry");
    };
    assert_eq!(p.update(Msg::Retry(actor, kind), &api).task.units(), 1);
    // The page that failed is asked for again: what was loaded stays.
    let _ = loaded(
        &mut p,
        PeopleKind::Followers,
        true,
        Ok(page(vec![author("bob", json!({}))], None)),
    );
    assert_eq!(p.authors_mut().count(), 2);
}

#[test]
fn muted_and_blocked_rows_undo_from_the_same_row() {
    let mut p = People::default();
    let _ = loaded(
        &mut p,
        PeopleKind::Muted,
        false,
        Ok(page(
            vec![
                author("ann", json!({"muted": true})),
                author("bob", json!({})),
            ],
            None,
        )),
    );
    let mut ui = simulator(
        View {
            people: &p,
            actor: ME,
            kind: PeopleKind::Muted,
            pictures: &NoPictures,
            badges: true,
        }
        .view(),
    );
    ui.click("Unmute").unwrap();
    ui.click("Mute").unwrap();
    ui.click("bob").unwrap();
    let msgs: Vec<Msg> = ui.into_messages().collect();
    assert!(matches!(
        &msgs[..],
        [Msg::Mute(a, false), Msg::Mute(b, true), Msg::Profile(c)]
            if a.as_str() == "did:plc:ann" && b.as_str() == "did:plc:bob" && c.as_str() == "did:plc:bob"
    ));
    let api = Fake;
    for m in msgs {
        let events = p.update(m, &api).events;
        assert_eq!(events.len(), 1);
    }
}

#[test]
fn an_empty_list_says_so() {
    let mut p = People::default();
    let _ = loaded(
        &mut p,
        PeopleKind::Blocked,
        false,
        Ok(page(Vec::new(), None)),
    );
    let mut ui = simulator(
        View {
            people: &p,
            actor: ME,
            kind: PeopleKind::Blocked,
            pictures: &NoPictures,
            badges: true,
        }
        .view(),
    );
    assert!(ui.find("You haven't blocked anyone.").is_ok());
}
