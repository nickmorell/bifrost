//! Parses real responses from Bluesky's public AppView into BiFrost's model types.
//! Hits the network, so it is opt-in: `cargo test -- --ignored`.

#![allow(clippy::unwrap_used, reason = "a failed request should fail the test")]

use bifrost::model::*;
use serde_json::Value;

const APPVIEW: &str = "https://public.api.bsky.app/xrpc";

async fn get(path: &str) -> Value {
    reqwest::get(format!("{APPVIEW}/{path}"))
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "hits the public Bluesky AppView"]
async fn author_feed_parses() {
    let v = get("app.bsky.feed.getAuthorFeed?actor=bsky.app&limit=50").await;
    let raw = v["feed"].as_array().unwrap();
    let parsed: Vec<FeedItem> = raw
        .iter()
        .map(|i| serde_json::from_value(i.clone()).unwrap_or_else(|e| panic!("{e}: {i}")))
        .collect();
    assert_eq!(parsed.len(), raw.len());
    assert!(parsed.iter().any(|i| !i.post.record.text.is_empty()));
}

#[tokio::test]
#[ignore = "hits the public Bluesky AppView"]
async fn thread_and_profile_parse() {
    let feed = get("app.bsky.feed.getAuthorFeed?actor=bsky.app&limit=5").await;
    let uri = feed["feed"][0]["post"]["uri"].as_str().unwrap();
    let t = get(&format!("app.bsky.feed.getPostThread?uri={uri}&depth=6")).await;
    let node: ThreadNode = serde_json::from_value(t["thread"].clone()).unwrap();
    assert!(matches!(node, ThreadNode::Post(_)));

    let p = get("app.bsky.actor.getProfile?actor=bsky.app").await;
    let profile: Profile = serde_json::from_value(p).unwrap();
    assert_eq!(profile.handle, "bsky.app");
    assert!(profile.followers_count > 0);
}

#[tokio::test]
#[ignore = "hits the public Bluesky AppView"]
async fn custom_feed_parses() {
    // Bluesky's "Discover" feed.
    let v = get("app.bsky.feed.getFeed?feed=at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot&limit=50").await;
    let raw = v["feed"].as_array().unwrap();
    let ok = raw
        .iter()
        .filter(|i| serde_json::from_value::<FeedItem>((*i).clone()).is_ok())
        .count();
    assert_eq!(ok, raw.len(), "every Discover item should parse");
}
