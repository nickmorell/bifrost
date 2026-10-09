use super::*;
use crate::net::fake::Fake;

/// The first stream of a real getLiveUsers answer, shortened.
const LIVE: &str = r#"{"streams":[{"$type":"place.stream.livestream#livestreamView",
 "author":{"did":"did:plc:64ah","handle":"monarkie.blacksky.app"},
 "cid":"bafy","indexedAt":"2026-10-07T21:32:03Z",
 "record":{"$type":"place.stream.livestream",
   "activity":{"$type":"place.stream.defs#activityGame","name":"Star Citizen","uri":"at://x/g/1"},
   "createdAt":"2026-10-07T21:03:41Z",
   "tags":["music","lang:en"],
   "thumb":{"$type":"blob","ref":{"$link":"bafkthumb"},"mimeType":"image/jpeg","size":7},
   "title":"Local Man Is Flying Around"},
 "uri":"at://did:plc:64ah/place.stream.livestream/3mxc",
 "viewerCount":{"$type":"place.stream.livestream#viewerCount","count":2}},
 {"$type":"place.stream.livestream#livestreamView","author":{"did":"did:plc:2zmx","handle":"iame.li"},
  "record":{"activity":{"$type":"place.stream.defs#activityLabel","label":"software_dev"},
   "createdAt":"2026-10-07T20:00:00Z","title":"New TLDs"},
  "uri":"at://did:plc:2zmx/place.stream.livestream/3abc"},
 {"broken":true}]}"#;

#[tokio::test]
async fn reads_who_is_live() {
    let net = Fake::default().reply(200, LIVE);
    let streams = Node(&net).live_now().await.unwrap();
    assert_eq!(streams.len(), 2, "the broken one is left out");
    let s = &streams[0];
    assert_eq!(s.did.as_str(), "did:plc:64ah");
    assert_eq!(s.handle, "monarkie.blacksky.app");
    assert_eq!(s.title, "Local Man Is Flying Around");
    assert_eq!(s.viewers, Some(2));
    assert_eq!(s.activity.as_deref(), Some("Star Citizen"));
    assert_eq!(s.tags, ["music"]);
    assert_eq!(
        s.thumb.as_deref(),
        Some("https://cdn.bsky.app/img/feed_thumbnail/plain/did:plc:64ah/bafkthumb@jpeg")
    );
    assert_eq!(streams[1].activity.as_deref(), Some("Software dev"));
    assert_eq!(streams[1].viewers, None);
    assert_eq!(
        net.seen()[0].url.as_str(),
        "https://stream.place/xrpc/place.stream.live.getLiveUsers?limit=50"
    );
}

#[tokio::test]
async fn someone_who_never_streamed_has_no_latest_stream() {
    let net = Fake::default().reply(404, "{}");
    assert_eq!(Node(&net).latest("bob.test").await.unwrap(), None);
    assert_eq!(net.seen()[0].url.path(), "/api/livestream/bob.test");

    let one = r#"{"author":{"did":"did:plc:a","handle":"a.test"},"uri":"at://did:plc:a/place.stream.livestream/1",
        "record":{"title":"Done","createdAt":"2026-10-01T00:00:00Z","endedAt":"2026-10-01T01:00:00Z"}}"#;
    let net = Fake::default().reply(200, one);
    let s = Node(&net).latest("a.test").await.unwrap().unwrap();
    assert_eq!(s.ended.as_deref(), Some("2026-10-01T01:00:00Z"));

    let net = Fake::default().reply(500, "{}");
    assert!(Node(&net).latest("a.test").await.is_err());
}

#[tokio::test]
async fn reads_past_streams() {
    let page = r#"{"videos":[{"uri":"at://did:plc:a/place.stream.video/1",
        "record":{"title":"Big Walk","createdAt":"2026-10-01T00:00:00Z"},
        "viewCounts":{"durationMs":3725000}}],"cursor":"next"}"#;
    let net = Fake::default().reply(200, page);
    let got = Node(&net).past("a.test", Some("c1")).await.unwrap();
    assert_eq!(got.cursor.as_deref(), Some("next"));
    assert_eq!(got.items[0].title, "Big Walk");
    assert_eq!(got.items[0].seconds, Some(3725));
    let q = net.seen()[0].url.query().unwrap_or_default().to_string();
    assert_eq!(q, "repo=a.test&limit=25&cursor=c1");
}

#[test]
fn chat_messages_come_with_their_author_and_colour() {
    let msg = r#"{"$type":"place.stream.chat.defs#messageView","uri":"at://did:plc:c/place.stream.chat.message/3mx","cid":"b",
     "author":{"did":"did:plc:c","handle":"alice.bsky.social","displayName":"Alice"},
     "record":{"$type":"place.stream.chat.message","text":"hi @iame.li","createdAt":"2026-10-07T21:40:00.000Z","streamer":"did:plc:2zmx",
       "facets":[{"index":{"byteStart":3,"byteEnd":11},"features":[{"$type":"app.bsky.richtext.facet#mention","did":"did:plc:2zmx"}]}]},
     "chatProfile":{"$type":"place.stream.chat.profile","color":{"red":255,"green":120,"blue":0}},
     "replyTo":{"uri":"at://x/y/z","author":{"did":"did:plc:d","handle":"dan.test"},"record":{"text":"hello"}}}"#;
    let Some(LiveEvent::Chat(c)) = event(msg) else {
        panic!("not a chat message");
    };
    assert_eq!(c.author.name(), "Alice");
    assert_eq!(c.body.text, "hi @iame.li");
    assert_eq!(c.body.facets.len(), 1);
    assert_eq!(c.color, Some([255, 120, 0]));
    assert_eq!(c.reply_to, Some(("dan.test".into(), "hello".into())));

    let plain = msg.replace(r#""chatProfile":{"$type":"place.stream.chat.profile","color":{"red":255,"green":120,"blue":0}},"#, "");
    let Some(LiveEvent::Chat(c)) = event(&plain) else {
        panic!("not a chat message");
    };
    assert_eq!(c.color, None);
}

#[test]
fn reads_the_other_happenings() {
    let ev = |s: &str| event(s);
    assert!(matches!(
        ev(r#"{"$type":"place.stream.livestream#viewerCount","count":18}"#),
        Some(LiveEvent::Viewers(18))
    ));
    assert!(matches!(
        ev(r#"{"$type":"place.stream.chat.defs#messageView","uri":"at://a/b/c","deleted":true}"#),
        Some(LiveEvent::Removed(u)) if u.as_str() == "at://a/b/c"
    ));
    assert!(matches!(
        ev(r#"{"$type":"place.stream.chat.gate","hiddenMessage":"at://a/b/d"}"#),
        Some(LiveEvent::Removed(u)) if u.as_str() == "at://a/b/d"
    ));
    assert!(matches!(
        ev(r#"{"$type":"place.stream.defs#blockView","record":{"subject":"did:plc:troll"}}"#),
        Some(LiveEvent::Blocked(d)) if d.as_str() == "did:plc:troll"
    ));
    assert!(matches!(
        ev(r#"{"$type":"place.stream.chat.pinnedRecord","deleted":true}"#),
        Some(LiveEvent::Pinned(None))
    ));
    assert!(ev(r#"{"$type":"place.stream.segment"}"#).is_none());
    assert!(ev("not json").is_none());
}

#[test]
fn addresses() {
    let did: Did = "did:plc:2zmx".into();
    assert_eq!(
        live_playlist(&did),
        "https://stream.place/xrpc/place.stream.playback.getLivePlaylist?streamer=did%3Aplc%3A2zmx"
    );
    let vod: AtUri = "at://did:plc:a/place.stream.video/1".into();
    assert!(past_playlist(&vod).contains("uri=at%3A%2F%2Fdid%3Aplc%3Aa%2Fplace.stream.video%2F1"));
    assert_eq!(
        streamer_in("https://stream.place/iame.li").as_deref(),
        Some("iame.li")
    );
    assert_eq!(
        streamer_in("https://stream.place/did:plc:64ah").as_deref(),
        Some("did:plc:64ah")
    );
    assert_eq!(streamer_in("https://stream.place/about"), None);
    assert_eq!(streamer_in("https://stream.place/a.b/more"), None);
    assert_eq!(streamer_in("https://example.com/iame.li"), None);
}
