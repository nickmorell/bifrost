//! The chat feature against a fake Bluesky.

mod fake;

use super::state::merge;
use super::*;
use crate::api::ApiError;
use crate::components::Picture;
use crate::features::Pictures;
use crate::model::{Author, ChatMessage, Convo, Did, Page, PostRecord, QuotedPost};
use fake::Fake;
use iced::Element;
use iced::widget::text;
use iced_test::simulator;
use serde_json::json;

const ME: &str = "did:plc:me";

/// The app as far as the views need it: no pictures, posts drawn as plain text.
struct App {
    browser: bool,
}

impl Pictures for App {
    fn picture(&self, _: Option<&str>) -> Option<Picture> {
        None
    }
}

impl Host<Msg> for App {
    fn rich_text<'a>(&'a self, body: &'a PostRecord) -> Element<'a, Msg> {
        text(&body.text).into()
    }
    fn quoted_post<'a>(&'a self, _: &'a QuotedPost) -> Element<'a, Msg> {
        text("A quoted post").into()
    }
    fn browser_sign_in(&self) -> bool {
        self.browser
    }
}

fn view<'a>(chats: &'a Chats, app: &'a App, me: &'a Did) -> View<'a, App, Msg> {
    View {
        chats,
        host: app,
        me,
        lift: |m| m,
    }
}

fn msg(id: &str, sender: &str, at: &str) -> ChatMessage {
    serde_json::from_value(json!({
        "$type": "chat.bsky.convo.defs#messageView",
        "id": id, "rev": id, "sender": {"did": sender}, "sentAt": at, "text": id
    }))
    .unwrap()
}

fn author(did: &str) -> Author {
    serde_json::from_value(json!({"did": did, "handle": format!("{}.test", &did[8..])})).unwrap()
}

fn convo(id: &str, others: &[&str], status: &str) -> Convo {
    let mut members = vec![json!({"did": ME, "handle": "me.test"})];
    members.extend(
        others
            .iter()
            .map(|d| json!({"did": d, "handle": format!("{}.test", &d[8..]), "avatar": format!("https://a/{}", &d[8..])})),
    );
    serde_json::from_value(json!({"id": id, "members": members, "status": status})).unwrap()
}

/// Feeds a message through `update` with `open` on screen and returns the events.
fn events(chats: &mut Chats, open: Option<&str>, msg: Msg) -> Vec<Event> {
    let me = Did::from(ME);
    let ctx = Ctx { me: &me, open };
    chats.update(msg, &ctx, &Fake).events
}

#[test]
fn merging_pages_keeps_order_and_replaces_updates() {
    let mut all = vec![msg("a", "x", "2026-10-05T10:00:00Z")];
    // Pages arrive newest first.
    let added = merge(
        &mut all,
        vec![
            msg("c", "y", "2026-10-05T10:02:00Z"),
            msg("b", "x", "2026-10-05T10:01:00Z"),
            msg("a", "x", "2026-10-05T10:00:00Z"),
        ],
    );
    assert!(added);
    let ids: Vec<_> = all.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "c"]);
    assert!(!merge(
        &mut all,
        vec![msg("b", "x", "2026-10-05T10:01:00Z")]
    ));
    assert_eq!(all.len(), 3);
}

#[test]
fn opening_a_chat_asks_the_app_unless_it_is_on_screen() {
    let mut chats = Chats::default();
    let got = events(&mut chats, None, Msg::Open("c1".into()));
    assert!(matches!(&got[..], [Event::OpenChat(id)] if id == "c1"));
    assert!(events(&mut chats, Some("c1"), Msg::Open("c1".into())).is_empty());
}

#[test]
fn a_listed_page_is_kept_and_its_avatars_fetched() {
    let mut chats = Chats::default();
    let page = Page {
        items: vec![
            convo("c1", &["did:plc:ann"], "accepted"),
            convo("c2", &["did:plc:bob"], "accepted"),
        ],
        cursor: Some("next".into()),
    };
    let got = events(&mut chats, None, Msg::Listed(false, false, Ok(page)));
    assert!(matches!(&got[..], [Event::Images(urls)] if urls.len() == 2));
    assert_eq!(chats.accepted.ids, ["c1", "c2"]);
    assert!(chats.accepted.loaded);
    assert_eq!(chats.title("c1", &Did::from(ME)), "ann.test");
    assert_eq!(chats.title("nope", &Did::from(ME)), "Chat");
}

#[test]
fn an_expired_sign_in_is_reported_first() {
    let mut chats = Chats::default();
    let got = events(
        &mut chats,
        None,
        Msg::Sent("c1".into(), Err(ApiError::SessionExpired)),
    );
    assert!(matches!(&got[..], [Event::SessionExpired]));
}

#[test]
fn a_failed_send_keeps_the_draft_and_says_so() {
    let mut chats = Chats::default();
    events(&mut chats, None, Msg::Draft("c1".into(), "hi".into()));
    let got = events(
        &mut chats,
        None,
        Msg::Sent("c1".into(), Err(ApiError::Other("down".into()))),
    );
    assert!(matches!(&got[..], [Event::Toast(t)] if t.starts_with("Couldn't send.")));
    assert_eq!(chats.threads["c1"].draft, "hi");
}

#[test]
fn leaving_asks_first_and_goes_back_when_it_was_open() {
    let mut chats = Chats::default();
    events(&mut chats, Some("c1"), Msg::Leave("c1".into()));
    assert_eq!(chats.confirm_leave.as_deref(), Some("c1"));
    let got = events(&mut chats, Some("c1"), Msg::Left("c1".into(), Ok(())));
    assert!(matches!(&got[..], [Event::Back]));
}

#[test]
fn a_started_chat_opens() {
    let mut chats = Chats::default();
    events(&mut chats, None, Msg::NewChat);
    let got = events(
        &mut chats,
        None,
        Msg::Started(Ok(Box::new(convo("c9", &["did:plc:ann"], "accepted")))),
    );
    assert!(matches!(&got[..], [Event::OpenChat(id)] if id == "c9"));
    assert!(chats.new_chat.is_none());
    assert_eq!(chats.accepted.ids, ["c9"]);
}

#[test]
fn view_buttons_become_app_events() {
    let mut chats = Chats::default();
    let did = Did::from("did:plc:ann");
    let got = events(&mut chats, None, Msg::OpenProfile(did.clone()));
    assert!(matches!(&got[..], [Event::OpenProfile(d)] if *d == did));
    let got = events(&mut chats, None, Msg::Report(did.clone()));
    assert!(matches!(&got[..], [Event::Report(d)] if *d == did));
    let got = events(&mut chats, None, Msg::SignOut);
    assert!(matches!(&got[..], [Event::SignOut]));
}

#[test]
fn without_access_the_list_says_how_to_fix_it() {
    let chats = Chats {
        no_access: true,
        ..Chats::default()
    };
    let me = Did::from(ME);
    let browser = App { browser: true };
    let mut ui = simulator(view(&chats, &browser, &me).list());
    ui.click("Sign out").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::SignOut]
    ));
    let password = App { browser: false };
    let mut ui = simulator(view(&chats, &password, &me).list());
    assert!(ui.find("App password settings").is_ok());
}

#[test]
fn the_new_chat_dialog_counts_the_group() {
    let mut chats = Chats::default();
    events(&mut chats, None, Msg::NewChat);
    let me = Did::from(ME);
    let app = App { browser: false };
    let mut ui = simulator(view(&chats, &app, &me).new_chat(chats.new_chat.as_ref().unwrap()));
    assert!(
        ui.find("Pick one person for a chat, or several for a group.")
            .is_ok()
    );
    ui.click("Start chat").unwrap();
    assert_eq!(ui.into_messages().count(), 0, "nobody picked yet");

    for d in ["did:plc:ann", "did:plc:bob"] {
        events(&mut chats, None, Msg::NewPick(author(d)));
    }
    let mut ui = simulator(view(&chats, &app, &me).new_chat(chats.new_chat.as_ref().unwrap()));
    assert!(ui.find("A group chat with 3 people.").is_ok());
    ui.click("Start group chat").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::NewStart]
    ));
}

#[test]
fn a_chat_shows_its_messages_and_request_buttons() {
    let mut chats = Chats::default();
    let page = Page {
        items: vec![convo("c1", &["did:plc:ann"], "request")],
        cursor: None,
    };
    events(&mut chats, None, Msg::Listed(true, false, Ok(page)));
    let messages = Page {
        items: vec![msg("hello there", "did:plc:ann", "2026-10-05T10:00:00Z")],
        cursor: None,
    };
    events(
        &mut chats,
        Some("c1"),
        Msg::Messages("c1".into(), Fetch::First, Ok(messages)),
    );
    let me = Did::from(ME);
    let app = App { browser: false };
    let mut ui = simulator(view(&chats, &app, &me).chat("c1"));
    assert!(ui.find("hello there").is_ok());
    ui.click("Accept").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Accept(id)] if id == "c1"
    ));
}
