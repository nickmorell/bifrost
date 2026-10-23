//! Tests for the helpers in `api/mod.rs`.

use super::*;

#[test]
fn parses_at_uris() {
    assert_eq!(
        parse_at_uri("at://did:plc:a/app.bsky.feed.post/3k"),
        Some(("did:plc:a".into(), "app.bsky.feed.post".into(), "3k".into()))
    );
    assert_eq!(parse_at_uri("https://x"), None);
    assert_eq!(parse_at_uri("at://did:plc:a/app.bsky.feed.post"), None);
}

#[test]
fn lenient_items_skip_bad_entries() {
    let items: Vec<StrongRef> = lenient_items(vec![
        json!({"uri": "at://a", "cid": "c"}),
        json!({"nope": true}),
    ]);
    assert_eq!(items.len(), 1);
}

#[test]
fn profiles_keep_fields_atrium_does_not_know() {
    let raw = json!({
        "did": "did:plc:a", "handle": "a.test", "pronouns": "she/her",
        "links": [{"uri": "at://did:plc:a/app.bsky.actor.link/1", "cid": "c",
                   "url": "https://blog.test", "title": "Blog", "icon": "https://cdn/i.png"}]
    });
    let out: atrium_api::app::bsky::actor::get_profile::Output =
        serde_json::from_value(raw).unwrap();
    let profile: Profile = conv(&out).unwrap();
    assert_eq!(profile.pronouns(), Some("she/her"));
    assert_eq!(profile.links[0].label(), "Blog");
    assert_eq!(profile.links[0].icon.as_deref(), Some("https://cdn/i.png"));
}
