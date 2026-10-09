//! The deck: the columns a new deck starts with, adding, removing and moving
//! columns, what the column buttons ask for, and the deck and its dialog.

use super::*;
use crate::features;
use crate::model::{DeckColumn, DeckWidth, Did, FeedSource, ListInfo};
use iced::Element;
use iced::widget::text;
use iced_test::simulator;
use serde_json::json;

fn run(d: &mut Deck, msg: Msg, cols: &[DeckColumn]) -> features::Update<Msg, Event> {
    d.update(msg, cols)
}

fn feed(name: &str) -> DeckColumn {
    DeckColumn::Feed {
        uri: format!("at://did:plc:a/app.bsky.feed.generator/{name}").into(),
        name: name.into(),
    }
}

fn source(name: &str) -> FeedSource {
    FeedSource::Generator(format!("at://did:plc:a/app.bsky.feed.generator/{name}").into())
}

fn titles(cols: &[DeckColumn]) -> Vec<&str> {
    cols.iter().map(DeckColumn::title).collect()
}

#[test]
fn a_new_deck_starts_with_following_notifications_and_pinned_feeds() {
    let (cats, dogs, owls) = (source("cats"), source("dogs"), source("owls"));
    let pinned = [
        (&FeedSource::Following, "Following"),
        (&cats, "cats"),
        (&dogs, "dogs"),
        (&owls, "owls"),
    ];
    assert_eq!(
        titles(&starting(pinned)),
        ["Following", "Notifications", "cats", "dogs"]
    );
}

#[test]
fn adding_a_column_saves_and_loads_it_unless_it_is_there_or_the_deck_is_full() {
    let mut d = Deck::default();
    let cols = vec![DeckColumn::Following];
    let out = run(&mut d, Msg::OpenAdd, &cols);
    assert!(d.adding());
    assert!(matches!(&out.events[..], [Event::LoadLists]));

    let out = run(&mut d, Msg::Add(feed("cats")), &cols);
    assert!(!d.adding(), "adding closes the dialog");
    assert!(matches!(
        &out.events[..],
        [Event::Toast(t), Event::Columns(c), Event::Load(added, false)]
            if t == "Added cats to your columns." && titles(c) == ["Following", "cats"]
                && added.title() == "cats"
    ));

    let out = run(&mut d, Msg::Add(DeckColumn::Following), &cols);
    assert!(matches!(&out.events[..], [Event::Toast(t)] if t == "Following is already a column."));

    let full: Vec<_> = (0..MAX_COLUMNS).map(|i| feed(&i.to_string())).collect();
    let out = run(&mut d, Msg::Add(feed("cats")), &full);
    assert!(matches!(&out.events[..], [Event::Toast(t)] if t.starts_with("A deck holds up to")));
}

#[test]
fn columns_move_and_go_within_bounds() {
    let mut d = Deck::default();
    let cols = vec![DeckColumn::Following, feed("cats"), DeckColumn::Bookmarks];
    let moved = |d: &mut Deck, msg| match &run(d, msg, &cols).events[..] {
        [Event::Columns(c)] => Some(titles(c).join(",")),
        [] => None,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        moved(&mut d, Msg::Move(1, -1)).as_deref(),
        Some("cats,Following,Bookmarks")
    );
    assert_eq!(
        moved(&mut d, Msg::Move(1, 1)).as_deref(),
        Some("Following,Bookmarks,cats")
    );
    assert_eq!(moved(&mut d, Msg::Move(0, -1)), None);
    assert_eq!(moved(&mut d, Msg::Move(2, 1)), None);
    assert_eq!(
        moved(&mut d, Msg::Remove(0)).as_deref(),
        Some("cats,Bookmarks")
    );
    assert_eq!(moved(&mut d, Msg::Remove(3)), None);
}

#[test]
fn column_buttons_ask_for_their_column() {
    let mut d = Deck::default();
    let cols = vec![DeckColumn::Following, DeckColumn::Notifications];
    let out = run(&mut d, Msg::Refresh(1), &cols);
    assert!(matches!(
        &out.events[..],
        [Event::Load(DeckColumn::Notifications, true)]
    ));
    let out = run(&mut d, Msg::Top(0), &cols);
    assert!(matches!(
        &out.events[..],
        [Event::Top(DeckColumn::Following)]
    ));
    let out = run(&mut d, Msg::ShowNew(0), &cols);
    assert!(matches!(
        &out.events[..],
        [Event::ShowNew(DeckColumn::Following)]
    ));
    assert!(run(&mut d, Msg::Refresh(5), &cols).events.is_empty());
    let out = run(&mut d, Msg::Width(DeckWidth::Wide), &cols);
    assert!(matches!(&out.events[..], [Event::Width(DeckWidth::Wide)]));
}

#[test]
fn scroll_positions_are_named_by_column_not_place() {
    assert_eq!(scroll_id(&feed("cats")), scroll_id(&feed("cats")));
    assert_ne!(scroll_id(&feed("cats")), scroll_id(&feed("dogs")));
}

struct Plain;

impl<'a> Host<'a, Msg> for Plain {
    fn body(&self, c: &DeckColumn, _: &str) -> Element<'a, Msg> {
        text(format!("{} posts", c.title())).into()
    }
    fn pending(&self, c: &DeckColumn) -> usize {
        usize::from(matches!(c, DeckColumn::Following))
    }
    fn buttons(&self, _: &DeckColumn) -> Option<Element<'a, Msg>> {
        None
    }
}

fn deck(columns: Vec<DeckColumn>) -> Element<'static, Msg> {
    View {
        columns,
        width: DeckWidth::Normal,
        unread: 3,
        host: Plain,
        lift: |m| m,
    }
    .view()
}

#[test]
fn the_deck_shows_each_column_with_its_buttons() {
    let mut ui = simulator(deck(vec![DeckColumn::Following, DeckColumn::Notifications]));
    assert!(ui.find("Following posts").is_ok());
    assert!(ui.find("Notifications posts").is_ok());
    assert!(ui.find("3").is_ok(), "unread notifications");
    ui.click("1 new post").unwrap();
    ui.click("Notifications").unwrap();
    ui.click("Add column").unwrap();
    ui.click("One column").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [
            Msg::ShowNew(0),
            Msg::Top(1),
            Msg::OpenAdd,
            Msg::Toggle(false)
        ]
    ));
}

#[test]
fn a_full_deck_offers_no_more_columns() {
    let full = (0..MAX_COLUMNS).map(|i| feed(&i.to_string())).collect();
    let mut ui = simulator(deck(full));
    assert!(ui.find("Add column").is_err());
}

#[test]
fn the_dialog_offers_only_columns_not_there_yet() {
    let me = Did::from("did:plc:me");
    let cats = source("cats");
    let lists: Vec<ListInfo> = serde_json::from_value(json!([
        {"uri": "at://did:plc:me/app.bsky.graph.list/a", "name": "Friends",
         "purpose": "app.bsky.graph.defs#curatelist"},
        {"uri": "at://did:plc:me/app.bsky.graph.list/b", "name": "Blocked",
         "purpose": "app.bsky.graph.defs#modlist"},
    ]))
    .unwrap();
    let dialog = AddColumn {
        have: vec![DeckColumn::Following],
        me: Some(&me),
        pinned: vec![(&FeedSource::Following, "Following"), (&cats, "cats")],
        lists: &lists,
        width: DeckWidth::Normal,
    };
    let mut ui = simulator(dialog.view());
    assert!(ui.find("Posts from people you follow").is_err());
    assert!(
        ui.find("Blocked").is_err(),
        "moderation lists aren't timelines"
    );
    ui.click("Your posts").unwrap();
    ui.click("cats").unwrap();
    ui.click("Friends").unwrap();
    ui.click("Wide").unwrap();
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(matches!(
        &msgs[..],
        [
            Msg::Add(DeckColumn::Profile { .. }),
            Msg::Add(DeckColumn::Feed { .. }),
            Msg::Add(DeckColumn::List { .. }),
            Msg::Width(DeckWidth::Wide)
        ]
    ));
}
