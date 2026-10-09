//! The lists feature against a fake Bluesky.

mod fake;

use super::effects;
use super::view::pack::pack_web_url;
use super::*;
use crate::api::ApiError;
use crate::components::Picture;
use crate::features::Pictures;
use crate::model::{AtUri, Author, Did, ListDetails, ListItem, ListPage, Page};
use fake::Fake;
use iced::Element;
use iced_test::simulator;
use serde_json::json;

const ME: &str = "did:plc:me";
const LIST: &str = "at://did:plc:me/app.bsky.graph.list/l1";
const PACK: &str = "at://did:plc:me/app.bsky.graph.starterpack/p1";

/// Pictures that never load.
struct NoPictures;

impl Pictures for NoPictures {
    fn picture(&self, _: Option<&str>) -> Option<Picture> {
        None
    }
}

fn author(did: &str, following: bool) -> Author {
    serde_json::from_value(json!({
        "did": did,
        "handle": format!("{}.test", &did[8..]),
        "viewer": if following { json!({"following": "at://f"}) } else { json!({}) },
    }))
    .unwrap()
}

fn page(purpose: &str, creator: &str) -> Box<ListPage> {
    let list = serde_json::from_value(json!({
        "uri": LIST, "name": "Rust folks", "purpose": purpose, "avatar": "https://a/l.jpg",
        "creator": {"did": creator, "handle": "c.test"}, "listItemCount": 1,
    }))
    .unwrap();
    let items = vec![ListItem {
        uri: AtUri::from("at://item/1"),
        subject: author("did:plc:ann", false),
    }];
    Box::new(ListPage {
        list,
        items: Page {
            items,
            cursor: None,
        },
    })
}

fn ctx(me: &Did) -> Ctx<'_> {
    Ctx {
        me,
        my_name: Some("Nick"),
    }
}

/// Feeds a message through `update` and returns the events, in order.
fn events(lists: &mut Lists, msg: Msg) -> Vec<Event> {
    let me = Did::from(ME);
    lists.update(msg, &ctx(&me), &Fake::default()).events
}

#[tokio::test]
async fn follow_all_skips_people_already_followed_and_you() {
    let fake = Fake {
        members: vec![
            ListItem {
                uri: AtUri::from("at://i/1"),
                subject: author("did:plc:ann", false),
            },
            ListItem {
                uri: AtUri::from("at://i/2"),
                subject: author("did:plc:bob", true),
            },
            ListItem {
                uri: AtUri::from("at://i/3"),
                subject: author(ME, false),
            },
        ],
        ..Fake::default()
    };
    let done = effects::follow_all(fake.clone(), AtUri::from(LIST), Did::from(ME))
        .await
        .unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(
        fake.calls(),
        [format!("members {LIST}"), "follow did:plc:ann".into()]
    );
}

#[tokio::test]
async fn saving_makes_or_changes_the_right_thing() {
    let fake = Fake::default();
    let details = ListDetails {
        name: "Mine".into(),
        ..ListDetails::default()
    };
    let save = |kind| {
        effects::save(
            fake.clone(),
            kind,
            details.clone(),
            vec![AtUri::from("at://f")],
        )
    };
    assert_eq!(save(Editing::NewList).await.unwrap(), LIST);
    assert_eq!(save(Editing::NewPack).await.unwrap(), PACK);
    save(Editing::List(AtUri::from(LIST))).await.unwrap();
    let pack = Editing::Pack {
        uri: AtUri::from(PACK),
        list: None,
    };
    save(pack).await.unwrap();
    assert_eq!(
        fake.calls(),
        [
            "create list Mine".to_string(),
            "create pack Mine".into(),
            format!("edit list {LIST}"),
            format!("edit pack {PACK} with 1 feeds"),
        ]
    );
}

#[tokio::test]
async fn deleting_blocking_and_toggling_pick_the_right_call() {
    let fake = Fake::default();
    let list = AtUri::from(LIST);
    effects::delete(fake.clone(), AtUri::from(PACK), Some(list.clone()))
        .await
        .unwrap();
    effects::delete(fake.clone(), list.clone(), None)
        .await
        .unwrap();
    let block = effects::block(fake.clone(), list.clone(), None).await;
    assert_eq!(block.unwrap().as_deref(), Some("at://block"));
    let unblock = effects::block(fake.clone(), list.clone(), Some(AtUri::from("at://b")));
    assert!(unblock.await.unwrap().is_none());
    let on = effects::toggle(fake.clone(), list.clone(), Did::from("did:plc:ann"), None);
    assert!(on.await.unwrap().is_some());
    let calls = fake.calls();
    assert_eq!(calls[0], format!("delete pack {PACK} and Some(\"{LIST}\")"));
    assert_eq!(calls[1], format!("delete list {LIST}"));
    assert_eq!(calls[3], "delete at://b");
    assert_eq!(calls[4], format!("add did:plc:ann to {LIST}"));
}

#[test]
fn a_moderation_list_opens_on_its_people() {
    let mut lists = Lists::default();
    let uri = AtUri::from(LIST);
    let got = events(
        &mut lists,
        Msg::Loaded(uri.clone(), Ok(page(crate::model::MODLIST, ME)), false),
    );
    assert!(matches!(&got[..], [Event::Images(urls)] if urls[0] == "https://a/l.jpg"));
    assert_eq!(lists.screens[&uri].tab, ListTab::People);
    assert_eq!(lists.title(LIST), "Moderation list");
    assert!(lists.timeline(&uri).is_none());
}

#[test]
fn saving_a_new_list_opens_it() {
    let mut lists = Lists::default();
    lists.actor_packs.insert(ME.into(), Ok(Vec::new()));
    events(&mut lists, Msg::NewList(false));
    events(&mut lists, Msg::Name("x".repeat(100)));
    assert_eq!(lists.editor.as_ref().unwrap().name.len(), 64);
    let got = events(&mut lists, Msg::Saved(Ok(AtUri::from(LIST))));
    assert!(matches!(
        &got[..],
        [Event::MyListsChanged, Event::OpenList(u)] if u == LIST
    ));
    assert!(lists.editor.is_none());
    assert!(lists.actor_packs.is_empty());
}

#[test]
fn deleting_a_pack_tells_the_app() {
    let mut lists = Lists::default();
    let got = events(&mut lists, Msg::Deleted(AtUri::from(PACK), Ok(())));
    assert!(matches!(
        &got[..],
        [Event::MyListsChanged, Event::Toast(t), Event::Deleted(u)]
            if t == "Starter pack deleted." && u == PACK
    ));
}

#[test]
fn a_failed_change_is_reported() {
    let mut lists = Lists::default();
    let got = events(
        &mut lists,
        Msg::Muted(AtUri::from(LIST), true, Err(ApiError::Other("boom".into()))),
    );
    assert!(matches!(&got[..], [Event::Error(_)]));
}

#[test]
fn list_page_tabs_and_people() {
    let mut lists = Lists::default();
    let uri = AtUri::from(LIST);
    events(
        &mut lists,
        Msg::Loaded(
            uri.clone(),
            Ok(page(crate::model::CURATE, "did:plc:other")),
            false,
        ),
    );
    let me = Did::from(ME);
    let ListParts::Page { top, people } = lists.list_view(&uri, &NoPictures, &me) else {
        panic!("the list is loaded");
    };
    assert!(people.is_none(), "the Posts tab shows the app's timeline");
    drop(people);
    let view: Element<'_, Msg> = iced::widget::Column::with_children(top).into();
    let mut ui = simulator(view);
    assert!(ui.find("Rust folks").is_ok());
    assert!(ui.find("Edit list").is_err(), "not your list");
    ui.click("People").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Tab(_, ListTab::People)]
    ));

    events(&mut lists, Msg::Tab(uri.clone(), ListTab::People));
    let ListParts::Page {
        people: Some(people),
        ..
    } = lists.list_view(&uri, &NoPictures, &me)
    else {
        panic!("the People tab shows people");
    };
    let mut ui = simulator(people);
    ui.click("ann.test").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::OpenProfile(d)] if d == "did:plc:ann"
    ));
}

#[test]
fn the_editor_saves_only_with_a_name() {
    let mut lists = Lists::default();
    events(&mut lists, Msg::NewList(true));
    let mut ui = simulator(editor(lists.editor.as_ref().unwrap(), &[]));
    assert!(ui.find("New moderation list").is_ok());
    ui.click("Save").unwrap();
    assert_eq!(ui.into_messages().count(), 0);

    events(&mut lists, Msg::Name("Spam".into()));
    let mut ui = simulator(editor(lists.editor.as_ref().unwrap(), &[]));
    ui.click("Save").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Save]
    ));
}

#[test]
fn pack_links() {
    assert_eq!(
        pack_web_url("at://did:plc:a/app.bsky.graph.starterpack/3k", "a.test"),
        "https://bsky.app/starter-pack/a.test/3k"
    );
    assert!(state::is_pack(PACK));
    assert!(!state::is_pack(LIST));
}

#[test]
fn starter_pack_embeds() {
    use crate::model::EmbeddedRecord;
    let v = json!({
        "$type": "app.bsky.graph.defs#starterPackViewBasic",
        "uri": "at://did:plc:a/app.bsky.graph.starterpack/3k",
        "cid": "c",
        "record": {"$type": "app.bsky.graph.starterpack", "name": "Rust folks", "list": "at://x"},
        "creator": {"did": "did:plc:a", "handle": "a.test"},
        "listItemCount": 12,
        "indexedAt": "2024-01-01T00:00:00Z"
    });
    let r: EmbeddedRecord = serde_json::from_value(v).unwrap();
    assert!(matches!(r, EmbeddedRecord::StarterPack { record, .. } if record.name == "Rust folks"));
}
