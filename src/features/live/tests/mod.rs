//! The Live feature against a fake Streamplace.

mod fake;

use super::*;
use crate::api::ApiError;
use crate::api::service::Channel as Info;
use crate::components::Picture;
use crate::features::Pictures;
use crate::model::{
    Author, Did, LiveChat, LiveEvent, LiveStream, Page, PastStream, PostRecord, Profile,
};
use fake::Fake;
use iced::Element;
use iced::widget::text;
use iced_test::simulator;
use serde_json::json;

const DID: &str = "did:plc:iame";
const NOW: &str = "2026-10-07T21:40:00Z";

/// An app that draws nothing but text, playing whatever `playing` says.
struct App {
    playing: Option<String>,
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
    fn playing(&self, key: &str) -> bool {
        self.playing.as_deref() == Some(key)
    }
    fn player<'a>(
        &'a self,
        key: &str,
        _: Option<&'a str>,
        _: Msg,
        _: f32,
        _: f32,
    ) -> Element<'a, Msg> {
        text(if self.playing(key) {
            "Playing"
        } else {
            "Poster"
        })
        .into()
    }
}

fn view<'a>(live: &'a Live, app: &'a App) -> View<'a, App, Msg> {
    View {
        live,
        host: app,
        now: NOW.into(),
        lift: |m| m,
    }
}

fn events(live: &mut Live, msg: Msg) -> Vec<Event> {
    live.update(msg, &Fake).events
}

/// A profile streaming on Streamplace until far in the future, or not at all.
fn profile(live: bool) -> Profile {
    let status = live.then(|| {
        json!({"status": "app.bsky.actor.status#live", "expiresAt": "2999-01-01T00:00:00Z",
            "embed": {"external": {"uri": "https://stream.place/iame.li", "title": "Coding"}}})
    });
    serde_json::from_value(json!({
        "did": DID, "handle": "iame.li", "displayName": "Eli", "avatar": "https://a/eli",
        "status": status,
    }))
    .unwrap()
}

fn stream(ended: Option<&str>) -> LiveStream {
    LiveStream {
        uri: "at://did:plc:iame/place.stream.livestream/1".into(),
        did: DID.into(),
        handle: "iame.li".into(),
        title: "New TLDs".into(),
        started: "2026-10-07T20:00:00Z".into(),
        ended: ended.map(str::to_string),
        viewers: Some(12),
        thumb: Some("https://stream.place/thumb".into()),
        activity: Some("Software dev".into()),
        tags: Vec::new(),
    }
}

fn author(did: &str) -> Author {
    serde_json::from_value(json!({"did": did, "handle": format!("{}.test", &did[8..])})).unwrap()
}

fn chat(id: &str, from: &str, at: &str) -> LiveChat {
    LiveChat {
        uri: format!("at://{from}/place.stream.chat.message/{id}")
            .as_str()
            .into(),
        author: author(from),
        body: PostRecord {
            text: id.into(),
            created_at: at.into(),
            ..PostRecord::default()
        },
        color: Some([255, 80, 80]),
        reply_to: None,
    }
}

/// A live streamer's page, opened as "iame.li" and loaded.
fn opened(live_now: bool) -> (Live, Vec<Event>) {
    let mut live = Live::default();
    let _ = events(&mut live, Msg::Open("iame.li".into()));
    let info = Info {
        profile: profile(live_now),
        latest: Some(stream(None)),
    };
    let got = events(&mut live, Msg::Loaded("iame.li".into(), Ok(Box::new(info))));
    (live, got)
}

fn texts(live: &Live) -> Vec<String> {
    live.channels["iame.li"]
        .chat
        .iter()
        .map(|m| m.body.text.clone())
        .collect()
}

#[test]
fn the_board_keeps_streams_and_fetches_their_pictures() {
    let mut live = Live::default();
    let got = events(
        &mut live,
        Msg::Board(Ok((vec![stream(None)], vec![author(DID)]))),
    );
    assert!(matches!(&got[..], [Event::Pictures(urls)] if urls == &["https://stream.place/thumb"]));
    assert_eq!(live.board.streams.len(), 1);
    assert!(live.board.loaded);
    assert!(
        events(&mut live, Msg::OpenBoard).is_empty(),
        "already loaded"
    );
}

#[test]
fn a_live_streamer_starts_playing_as_the_page_opens() {
    let (live, got) = opened(true);
    let play = got.iter().find_map(|e| match e {
        Event::Play { key, playlist, web } => Some((key, playlist, web)),
        _ => None,
    });
    let (key, playlist, web) = play.expect("plays");
    assert_eq!(key, "live:did:plc:iame");
    assert!(playlist.contains("getLivePlaylist?streamer=did%3Aplc%3Aiame"));
    assert_eq!(web, "https://stream.place/iame.li");
    assert_eq!(live.did_of("iame.li").map(Did::as_str), Some(DID));
    assert_eq!(live.title("iame.li"), "Eli");
    assert!(live.is_live("iame.li", NOW));
}

#[test]
fn an_offline_streamer_doesnt_play() {
    let (live, got) = opened(false);
    assert!(!got.iter().any(|e| matches!(e, Event::Play { .. })));
    assert!(!live.is_live("iame.li", NOW));
}

#[test]
fn chat_stays_in_time_order_without_copies() {
    let (mut live, _) = opened(true);
    let did = Did::from(DID);
    for m in [
        chat("b", "did:plc:bob", "2026-10-07T21:02:00Z"),
        chat("a", "did:plc:ann", "2026-10-07T21:01:00Z"),
        chat("c", "did:plc:ann", "2026-10-07T21:03:00Z"),
        chat("b", "did:plc:bob", "2026-10-07T21:02:00Z"),
    ] {
        let _ = events(
            &mut live,
            Msg::Happened(did.clone(), Box::new(LiveEvent::Chat(m))),
        );
    }
    assert_eq!(texts(&live), ["a", "b", "c"]);

    let gone = chat("b", "did:plc:bob", "").uri;
    let _ = events(
        &mut live,
        Msg::Happened(did.clone(), Box::new(LiveEvent::Removed(gone))),
    );
    assert_eq!(texts(&live), ["a", "c"]);

    let _ = events(
        &mut live,
        Msg::Happened(
            did.clone(),
            Box::new(LiveEvent::Blocked("did:plc:ann".into())),
        ),
    );
    assert!(texts(&live).is_empty());
    let again = chat("d", "did:plc:ann", "2026-10-07T21:04:00Z");
    let _ = events(
        &mut live,
        Msg::Happened(did, Box::new(LiveEvent::Chat(again))),
    );
    assert!(texts(&live).is_empty(), "blocked people stay out");
}

#[test]
fn old_chat_scrolls_away() {
    let (mut live, _) = opened(true);
    for i in 0..state::MAX_CHAT + 5 {
        let m = chat(
            &format!("{i:04}"),
            "did:plc:ann",
            &format!("2026-10-07T21:{:02}:{:02}Z", i / 60 % 60, i % 60),
        );
        let _ = events(
            &mut live,
            Msg::Happened(DID.into(), Box::new(LiveEvent::Chat(m))),
        );
    }
    let t = texts(&live);
    assert_eq!(t.len(), state::MAX_CHAT);
    assert_eq!(t[0], "0005");
}

#[test]
fn events_for_other_streams_are_ignored() {
    let (mut live, _) = opened(true);
    let m = chat("x", "did:plc:ann", NOW);
    let _ = events(
        &mut live,
        Msg::Happened("did:plc:else".into(), Box::new(LiveEvent::Chat(m))),
    );
    assert!(texts(&live).is_empty());
}

#[test]
fn sending_clears_the_draft_and_failures_say_why() {
    let (mut live, _) = opened(true);
    let _ = events(
        &mut live,
        Msg::Draft("iame.li".into(), "  hi there ".into()),
    );
    let _ = events(&mut live, Msg::Send("iame.li".into()));
    let c = &live.channels["iame.li"];
    assert!(c.sending);
    assert!(c.draft.is_empty());

    let got = events(
        &mut live,
        Msg::Sent("iame.li".into(), Err(ApiError::Other("Nope".into()))),
    );
    assert!(matches!(&got[..], [Event::Failed(_)]));
    let c = &live.channels["iame.li"];
    assert!(!c.sending);
    assert_eq!(c.send_error.as_deref(), Some("Nope"));
}

#[test]
fn empty_or_too_long_messages_arent_sent() {
    let (mut live, _) = opened(true);
    let _ = events(&mut live, Msg::Draft("iame.li".into(), "   ".into()));
    let _ = events(&mut live, Msg::Send("iame.li".into()));
    assert!(!live.channels["iame.li"].sending);
    let long = "x".repeat(MAX_MESSAGE + 1);
    let _ = events(&mut live, Msg::Draft("iame.li".into(), long));
    let _ = events(&mut live, Msg::Send("iame.li".into()));
    assert!(!live.channels["iame.li"].sending);
}

#[test]
fn past_pages_add_up() {
    let (mut live, _) = opened(false);
    let past = |id: &str| PastStream {
        uri: format!("at://v/{id}").as_str().into(),
        title: id.into(),
        created: NOW.into(),
        seconds: Some(90),
    };
    let page = |items, cursor: Option<&str>| {
        Ok(Page {
            items,
            cursor: cursor.map(str::to_string),
        })
    };
    let _ = events(
        &mut live,
        Msg::Past("iame.li".into(), false, page(vec![past("a")], Some("n"))),
    );
    let _ = events(
        &mut live,
        Msg::Past("iame.li".into(), true, page(vec![past("b")], None)),
    );
    let c = &live.channels["iame.li"];
    assert_eq!(c.past.len(), 2);
    assert!(c.past_cursor.is_none());
    assert!(events(&mut live, Msg::MorePast("iame.li".into())).is_empty());
}

#[test]
fn watching_a_past_stream_plays_it_under_the_streamers_key() {
    let (mut live, _) = opened(false);
    let got = events(
        &mut live,
        Msg::WatchPast("iame.li".into(), "at://v/1".into()),
    );
    assert!(matches!(&got[..], [Event::Play { key, playlist, .. }]
        if key == "live:did:plc:iame" && playlist.contains("getVideoPlaylist")));
}

#[test]
fn every_screen_draws() {
    let app = App { playing: None };
    let mut live = Live::default();
    let _ = events(&mut live, Msg::OpenBoard);
    let empty = Live::default();
    assert!(
        simulator(view(&live, &app).board())
            .snapshot(&iced::Theme::Dark)
            .is_ok()
    );
    assert!(
        simulator(view(&empty, &app).channel("nobody"))
            .snapshot(&iced::Theme::Light)
            .is_ok()
    );
    let _ = events(
        &mut live,
        Msg::Board(Ok((vec![stream(None)], vec![author(DID)]))),
    );
    let mut ui = simulator(view(&live, &app).board());
    assert!(ui.find("New TLDs").is_ok());

    let (mut on, _) = opened(true);
    let m = chat("hello", "did:plc:ann", NOW);
    let _ = events(
        &mut on,
        Msg::Happened(DID.into(), Box::new(LiveEvent::Chat(m))),
    );
    let mut ui = simulator(view(&on, &app).channel("iame.li"));
    assert!(ui.find("Poster").is_ok());
    assert!(ui.find("hello").is_ok());

    let (off, _) = opened(false);
    let mut ui = simulator(view(&off, &app).channel("iame.li"));
    assert!(ui.find("Offline").is_ok());
    let playing = App {
        playing: Some("live:did:plc:iame".into()),
    };
    let mut ui = simulator(view(&off, &playing).channel("iame.li"));
    assert!(
        ui.find("Playing").is_ok(),
        "a past stream plays on an offline page"
    );
}

#[test]
fn pictures_come_only_from_bluesky_and_streamplace() {
    let urls = [
        "https://cdn.bsky.app/img/avatar/plain/did:plc:a/b@jpeg",
        "https://stream.place/thumb.jpg",
        "https://cdn.stream.place/x.png",
        "https://tracker.example/pixel.png",
        "https://stream.place.evil.test/x.png",
        "http://cdn.bsky.app/img/x",
    ];
    let Event::Pictures(kept) = update::pictures(urls.map(String::from).to_vec()) else {
        panic!("not pictures");
    };
    assert_eq!(kept, &urls[..3]);
}
