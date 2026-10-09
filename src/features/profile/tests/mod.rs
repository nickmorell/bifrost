//! Profiles: what gets loaded when, what failures keep, the "Edit profile" dialog,
//! and the profile screen.

mod fake;

use super::*;
use crate::api::ApiError;
use crate::components::Picture;
use crate::features::Pictures;
use crate::model::{FeedGenerator, FeedSource, ListInfo, PeopleKind, Profile, StrongRef};
use fake::Fake;
use iced::Element;
use iced::widget::text;
use iced_test::simulator;
use serde_json::json;

const ANN: &str = "did:plc:ann";
const ME: &str = "did:plc:me";

fn profile(did: &str, viewer: serde_json::Value) -> Profile {
    serde_json::from_value(json!({
        "did": did,
        "handle": format!("{}.test", &did[8..]),
        "displayName": format!("Name of {}", &did[8..]),
        "description": "Hello there",
        "avatar": "https://a/avatar.jpg",
        "banner": "https://a/banner.jpg",
        "followersCount": 1200, "followsCount": 34, "postsCount": 5,
        "viewer": viewer,
    }))
    .unwrap()
}

fn feed(name: &str) -> FeedGenerator {
    FeedGenerator {
        uri: format!("at://{ANN}/app.bsky.feed.generator/{name}")
            .as_str()
            .into(),
        display_name: name.into(),
        avatar: Some(format!("https://a/{name}.jpg")),
        description: None,
    }
}

fn run(p: &mut Profiles, msg: Msg) -> features::Update<Msg, Event> {
    p.update(msg, &Ctx { me: None }, &Fake, &Fake)
}

fn events(p: &mut Profiles, msg: Msg) -> Vec<Event> {
    run(p, msg).events
}

fn loaded(p: &mut Profiles, actor: &str, profile: Profile) {
    let _ = run(p, Msg::Loaded(actor.into(), Box::new(Ok(profile))));
}

use crate::features;

#[test]
fn opening_a_profile_loads_it_and_its_tab_once() {
    let mut p = Profiles::default();
    assert_eq!(
        run(&mut p, Msg::Open(ANN.into(), Tab::Posts)).task.units(),
        1
    );
    assert_eq!(
        run(&mut p, Msg::Open(ANN.into(), Tab::Feeds)).task.units(),
        2,
        "the profile is still loading, and the feeds are needed too"
    );
    loaded(&mut p, ANN, profile(ANN, json!({})));
    let _ = run(&mut p, Msg::FeedsLoaded(ANN.into(), Ok(vec![feed("cats")])));
    assert_eq!(
        run(&mut p, Msg::Open(ANN.into(), Tab::Feeds)).task.units(),
        0
    );
    assert_eq!(
        run(&mut p, Msg::Open(ANN.into(), Tab::Lists)).task.units(),
        1,
        "only the lists"
    );
}

#[test]
fn what_loads_asks_for_its_pictures() {
    let mut p = Profiles::default();
    let ev = events(
        &mut p,
        Msg::Loaded(ANN.into(), Box::new(Ok(profile(ANN, json!({}))))),
    );
    assert!(
        matches!(&ev[..], [Event::Pictures(u)] if u == &["https://a/avatar.jpg", "https://a/banner.jpg"])
    );
    assert!(matches!(p.get(ANN), Some(Ok(pr)) if pr.did == ANN));

    let ev = events(&mut p, Msg::FeedsLoaded(ANN.into(), Ok(vec![feed("cats")])));
    assert!(matches!(&ev[..], [Event::Pictures(u)] if u == &["https://a/cats.jpg"]));
}

#[test]
fn failures_are_kept_in_words_until_tried_again() {
    let mut p = Profiles::default();
    let ev = events(
        &mut p,
        Msg::Loaded(
            ANN.into(),
            Box::new(Err(ApiError::Other("Profile not found".into()))),
        ),
    );
    assert!(matches!(&ev[..], [Event::Failed(_)]));
    assert!(matches!(p.get(ANN), Some(Err(_))));
    assert_eq!(
        run(&mut p, Msg::Open(ANN.into(), Tab::Posts)).task.units(),
        0
    );

    p.forget_failed(ANN);
    assert!(p.get(ANN).is_none());
    loaded(&mut p, ANN, profile(ANN, json!({})));
    p.forget_failed(ANN);
    assert!(p.get(ANN).is_some(), "a loaded profile stays");
}

#[test]
fn your_lists_load_until_they_have_loaded() {
    let mut p = Profiles::default();
    assert_eq!(
        run(&mut p, Msg::LoadLists(String::new())).task.units(),
        0,
        "not signed in"
    );
    assert_eq!(run(&mut p, Msg::LoadLists(ME.into())).task.units(), 1);
    let _ = run(
        &mut p,
        Msg::ListsLoaded(ME.into(), Err(ApiError::Other("down".into()))),
    );
    assert_eq!(
        run(&mut p, Msg::LoadLists(ME.into())).task.units(),
        1,
        "tried again"
    );
    let _ = run(
        &mut p,
        Msg::ListsLoaded(ME.into(), Ok(Vec::<ListInfo>::new())),
    );
    assert_eq!(run(&mut p, Msg::LoadLists(ME.into())).task.units(), 0);
    assert!(matches!(p.lists(ME), Some(Ok(l)) if l.is_empty()));
}

#[test]
fn refreshing_forgets_the_profile_and_what_the_tab_shows() {
    let mut p = Profiles::default();
    loaded(&mut p, ANN, profile(ANN, json!({})));
    let _ = run(&mut p, Msg::FeedsLoaded(ANN.into(), Ok(vec![feed("cats")])));
    let _ = run(&mut p, Msg::ListsLoaded(ANN.into(), Ok(vec![])));
    p.forget(ANN, Tab::Feeds);
    assert!(p.get(ANN).is_none());
    assert!(p.lists(ANN).is_some(), "the lists were not on screen");
    assert_eq!(
        run(&mut p, Msg::Open(ANN.into(), Tab::Feeds)).task.units(),
        2
    );
}

#[test]
fn trimming_keeps_wanted_accounts_once_there_are_too_many() {
    let mut p = Profiles::default();
    for did in ["did:plc:a", "did:plc:b", "did:plc:c"] {
        loaded(&mut p, did, profile(did, json!({})));
    }
    p.trim(3, |_| false);
    assert!(p.get("did:plc:a").is_some(), "not too many yet");
    p.trim(2, |did| did == "did:plc:b");
    assert!(p.get("did:plc:a").is_none());
    assert!(p.get("did:plc:b").is_some());
}

#[test]
fn accounts_opened_by_handle_are_found_and_changed_by_did() {
    let mut p = Profiles::default();
    loaded(&mut p, "ann.test", profile(ANN, json!({})));
    assert!(p.viewer(ANN).is_some_and(|v| !v.muted));
    p.with_actor(ANN, &mut |v| v.muted = true);
    assert!(p.viewer(ANN).is_some_and(|v| v.muted));

    let pin = StrongRef {
        uri: format!("at://{ANN}/app.bsky.feed.post/1").as_str().into(),
        cid: "c".into(),
    };
    p.set_pinned(ANN, Some(&pin));
    assert!(matches!(p.get("ann.test"), Some(Ok(pr)) if pr.pinned_post.is_some()));

    p.forget_account(ANN);
    assert!(p.get("ann.test").is_none(), "every copy goes");
}

#[test]
fn the_editor_opens_on_your_own_profile_and_closes_when_saved() {
    let mut p = Profiles::default();
    let _ = run(&mut p, Msg::Edit);
    assert!(p.editor().is_none(), "your profile has not loaded yet");

    let me = profile(ME, json!({}));
    let _ = p.update(Msg::Edit, &Ctx { me: Some(&me) }, &Fake, &Fake);
    assert!(p.editor().is_some());
    assert_eq!(run(&mut p, Msg::Editor(EditMsg::Save)).task.units(), 1);

    let ev = events(
        &mut p,
        Msg::Editor(EditMsg::Saved(Err(ApiError::Other("no".into())))),
    );
    assert!(ev.is_empty());
    assert!(p.editor().is_some(), "stays open to try again");

    let ev = events(&mut p, Msg::Editor(EditMsg::Saved(Ok(()))));
    assert!(matches!(&ev[..], [Event::Saved]));
    assert!(p.editor().is_none());
}

#[test]
fn the_editor_closes_when_signed_out_or_cancelled() {
    let me = profile(ME, json!({}));
    let mut p = Profiles::default();
    let _ = p.update(Msg::Edit, &Ctx { me: Some(&me) }, &Fake, &Fake);
    let ev = events(
        &mut p,
        Msg::Editor(EditMsg::Saved(Err(ApiError::SessionExpired))),
    );
    assert!(matches!(&ev[..], [Event::Failed(ApiError::SessionExpired)]));
    assert!(p.editor().is_none());

    let _ = p.update(Msg::Edit, &Ctx { me: Some(&me) }, &Fake, &Fake);
    let _ = run(&mut p, Msg::Editor(EditMsg::Cancel));
    assert!(p.editor().is_none());
}

/// The parts of the profile screen the app draws, as plain stand-ins.
struct Plain;

impl Pictures for Plain {
    fn picture(&self, _: Option<&str>) -> Option<Picture> {
        None
    }
}

impl Host<Msg> for Plain {
    fn posts<'a>(&'a self, actor: &str, tab: Tab) -> Element<'a, Msg> {
        text(format!("posts of {actor} on {tab:?}")).into()
    }
    fn starter_packs<'a>(&'a self, _: &str, empty: &'static str) -> Element<'a, Msg> {
        text(empty).into()
    }
    fn list_card<'a>(&'a self, list: &'a ListInfo) -> Element<'a, Msg> {
        text(&list.name).into()
    }
    fn linked_text<'a>(&'a self, t: &'a str) -> Element<'a, Msg> {
        text(t).into()
    }
    fn retry<'a>(&self, why: String) -> Element<'a, Msg> {
        text(why).into()
    }
}

fn screen<'a>(p: &'a Profiles, actor: &'a str, tab: Tab) -> Element<'a, Msg> {
    View {
        profiles: p,
        actor,
        tab,
        me: ME,
        can_edit: true,
        chat_opening: false,
        host: &Plain,
        lift: |m| m,
    }
    .body()
}

#[test]
fn someone_elses_profile_offers_to_follow_and_open_their_people() {
    let mut p = Profiles::default();
    loaded(&mut p, ANN, profile(ANN, json!({"followedBy": "at://f"})));
    let mut ui = simulator(screen(&p, ANN, Tab::Posts));
    for shown in [
        "Name of ann",
        "@ann.test",
        "Follows you",
        "1.2K",
        "Hello there",
    ] {
        assert!(ui.find(shown).is_ok(), "{shown} is missing");
    }
    assert!(ui.find(format!("posts of {ANN} on Posts").as_str()).is_ok());
    assert!(ui.find("Likes").is_err(), "only your own likes can be seen");
    assert!(ui.find("Edit profile").is_err());
    ui.click("Follow").unwrap();
    ui.click("followers").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Follow(d, true), Msg::People(a, PeopleKind::Followers)]
            if d.as_str() == ANN && a == ANN
    ));
}

#[test]
fn your_own_profile_can_be_edited_and_shows_likes() {
    let mut p = Profiles::default();
    loaded(&mut p, ME, profile(ME, json!({})));
    let mut ui = simulator(screen(&p, ME, Tab::Posts));
    assert!(ui.find("Likes").is_ok());
    assert!(ui.find("Follow").is_err());
    ui.click("Edit profile").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Edit]
    ));
}

#[test]
fn a_block_hides_the_posts() {
    let mut p = Profiles::default();
    loaded(&mut p, ANN, profile(ANN, json!({"blockedBy": true})));
    let mut ui = simulator(screen(&p, ANN, Tab::Posts));
    assert!(ui.find("Posts are hidden because of a block.").is_ok());
    assert!(
        ui.find(format!("posts of {ANN} on Posts").as_str())
            .is_err()
    );
}

#[test]
fn the_feeds_tab_lists_their_feeds_and_opens_one() {
    let mut p = Profiles::default();
    loaded(&mut p, ANN, profile(ANN, json!({})));
    assert!(
        simulator(screen(&p, ANN, Tab::Feeds))
            .find("Loading…")
            .is_ok()
    );
    let _ = run(&mut p, Msg::FeedsLoaded(ANN.into(), Ok(vec![feed("cats")])));
    let mut ui = simulator(screen(&p, ANN, Tab::Feeds));
    ui.click("cats").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::OpenFeed(FeedSource::Generator(_), name)] if name == "cats"
    ));
    let _ = run(&mut p, Msg::FeedsLoaded(ANN.into(), Ok(vec![])));
    assert!(
        simulator(screen(&p, ANN, Tab::Feeds))
            .find("No custom feeds yet.")
            .is_ok()
    );
}
