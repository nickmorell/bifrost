//! Your feeds against a fake Bluesky: Home tabs following your pinned feeds, names
//! looked up, and the Feeds screen.

mod fake;

use super::*;
use crate::components::Picture;
use crate::features::Pictures;
use crate::model::{FeedSource, PrefEdit, SavedFeed};
use fake::{Fake, generator};
use iced_test::simulator;

const CATS: &str = "at://did:plc:f/app.bsky.feed.generator/cats";
const DOGS: &str = "at://did:plc:f/app.bsky.feed.generator/dogs";
const GONE: &str = "at://did:plc:f/app.bsky.graph.list/gone";

fn feed(uri: &str) -> FeedSource {
    FeedSource::Generator(uri.into())
}

fn saved(pinned: &[FeedSource], other: &[FeedSource]) -> Vec<SavedFeed> {
    let mut out = vec![SavedFeed {
        source: FeedSource::Following,
        pinned: true,
    }];
    out.extend(pinned.iter().map(|s| SavedFeed {
        source: s.clone(),
        pinned: true,
    }));
    out.extend(other.iter().map(|s| SavedFeed {
        source: s.clone(),
        pinned: false,
    }));
    out
}

fn names(f: &Feeds) -> Vec<&str> {
    f.tabs().iter().map(|t| t.name.as_str()).collect()
}

struct NoPictures;

impl Pictures for NoPictures {
    fn picture(&self, _: Option<&str>) -> Option<Picture> {
        None
    }
}

#[tokio::test]
async fn tabs_follow_pinned_feeds_and_drop_deleted_ones() {
    let api = Fake {
        feeds: vec![(CATS, "Cats")],
        lists: Vec::new(),
    };
    let mut f = Feeds::default();
    let gone = FeedSource::List(GONE.into());
    let prefs = saved(&[feed(CATS), gone.clone()], &[feed(DOGS)]);
    let out = f.update(Msg::Reset, &prefs, &api);
    assert_eq!(out.task.units(), 1, "looks up the names");
    assert_eq!(names(&f), ["Following", "…", "…"]);
    let found = load::names(&api, vec![feed(CATS), gone]).await;
    let _ = f.update(Msg::Names(found), &prefs, &api);
    assert_eq!(names(&f), ["Following", "Cats"]);
}

#[test]
fn the_open_tab_stays_open_when_tabs_change() {
    let api = Fake::default();
    let mut f = Feeds::default();
    let both = saved(&[feed(CATS), feed(DOGS)], &[]);
    let _ = f.update(Msg::Reset, &both, &api);
    assert!(matches!(
        &f.update(Msg::Select(2), &both, &api).events[..],
        [Event::Selected]
    ));
    assert!(matches!(
        &f.update(Msg::Select(2), &both, &api).events[..],
        [Event::ShowNew]
    ));
    // Cats unpinned: Dogs is still open, now second.
    let dogs = saved(&[feed(DOGS)], &[feed(CATS)]);
    let _ = f.update(Msg::Sync, &dogs, &api);
    assert_eq!(f.active().map(|t| &t.source), Some(&feed(DOGS)));
    // Dogs gone too: back to Following.
    let none = saved(&[], &[]);
    let _ = f.update(Msg::Sync, &none, &api);
    assert_eq!(f.active().map(|t| &t.source), Some(&FeedSource::Following));
    assert!(
        f.update(Msg::Sync, &none, &api).task.units() == 0,
        "nothing to look up"
    );
}

#[tokio::test]
async fn the_feeds_screen_looks_up_names_once_and_suggestions() {
    let api = Fake {
        feeds: vec![(CATS, "Cats"), (DOGS, "Dogs")],
        lists: Vec::new(),
    };
    let mut f = Feeds::default();
    let prefs = saved(&[feed(CATS)], &[feed(DOGS)]);
    assert_eq!(
        f.update(Msg::Load { force: false }, &prefs, &api)
            .task
            .units(),
        2
    );
    let info = load::info(&api, vec![feed(CATS), feed(DOGS)]).await;
    let events = f.update(Msg::Info(info), &prefs, &api).events;
    assert!(matches!(&events[..], [Event::Pictures(urls)] if urls.len() == 2));
    assert_eq!(f.name(&feed(DOGS)), Some("Dogs"));
    let suggested = vec![generator(CATS, "Cats")];
    let _ = f.update(Msg::Suggested(Ok(suggested)), &prefs, &api);
    assert_eq!(f.suggested().len(), 1);
    // Names and suggestions are kept; loading again only checks for none missing.
    assert_eq!(
        f.update(Msg::Load { force: false }, &prefs, &api)
            .task
            .units(),
        1
    );
    assert_eq!(
        f.update(Msg::Suggest { force: true }, &prefs, &api)
            .task
            .units(),
        1
    );
}

#[test]
fn the_feeds_screen_pins_moves_and_opens_feeds() {
    let api = Fake::default();
    let mut f = Feeds::default();
    let prefs = saved(&[feed(CATS)], &[feed(DOGS)]);
    let _ = f.update(
        Msg::Info(vec![
            (feed(CATS), "Cats".into(), None),
            (feed(DOGS), "Dogs".into(), None),
        ]),
        &prefs,
        &api,
    );
    let _ = f.update(
        Msg::Suggested(Ok(vec![generator(DOGS, "Dogs")])),
        &prefs,
        &api,
    );
    let mut ui = simulator(
        Screen {
            feeds: &f,
            saved: &prefs,
            pictures: &NoPictures,
        }
        .view(),
    );
    ui.click("Unpin").unwrap();
    ui.click("Remove").unwrap();
    ui.click("Cats").unwrap();
    ui.click("Following").unwrap();
    let msgs: Vec<Msg> = ui.into_messages().collect();
    assert!(matches!(
        &msgs[..],
        [
            Msg::Edit(PrefEdit::PinFeed(a, false)),
            Msg::Edit(PrefEdit::RemoveFeed(b)),
            Msg::Open(c, name),
            Msg::Home,
        ] if *a == feed(CATS) && *b == feed(DOGS) && *c == feed(CATS) && name == "Cats"
    ));
}

#[test]
fn the_home_tabs_select_and_offer_more() {
    let api = Fake::default();
    let mut f = Feeds::default();
    f.following_only();
    let _ = f.update(Msg::Names(Vec::new()), &[], &api);
    let mut ui = simulator(Tabs { feeds: &f }.view());
    ui.click("Following").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Select(0)]
    ));
}
