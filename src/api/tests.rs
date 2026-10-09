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
