use super::*;
use serde_json::json;

fn post(extra: serde_json::Value) -> PostView {
    let mut p = json!({
        "uri": "at://did:plc:a/app.bsky.feed.post/1",
        "cid": "c",
        "author": { "did": "did:plc:a", "handle": "alice.test", "displayName": "Alice" },
        "record": {
            "text": "Reading about Rust on a rainy day",
            "createdAt": "2026-03-04T10:00:00Z",
            "facets": [{
                "index": {"byteStart": 0, "byteEnd": 1},
                "features": [{"$type": "app.bsky.richtext.facet#link", "uri": "https://blog.rust-lang.org/x"}]
            }]
        },
        "likeCount": 3,
        "indexedAt": "2026-03-04T10:00:00Z"
    });
    for (k, v) in extra.as_object().unwrap() {
        p[k] = v.clone();
    }
    serde_json::from_value(p).unwrap()
}

fn kept() -> Kept {
    Kept::of(
        &post(json!({
            "embed": {
                "$type": "app.bsky.embed.external#view",
                "external": {"uri": "https://www.example.com/a", "title": "Cats", "description": "All about cats"}
            },
            "viewer": {"like": "at://did:plc:me/app.bsky.feed.like/1", "bookmarked": true}
        })),
        "did:plc:me",
        "2026-03-05T00:00:00Z",
    )
}

#[test]
fn keeps_what_search_and_reading_need() {
    let k = kept();
    assert_eq!(k.name, "Alice");
    assert_eq!(
        k.links,
        ["https://blog.rust-lang.org/x", "https://www.example.com/a"]
    );
    assert_eq!(k.card.as_ref().unwrap().title, "Cats");
    assert!(k.liked && k.saved && !k.mine);
    assert_eq!(k.likes, 3);
    let back: Kept = serde_json::from_str(&serde_json::to_string(&k).unwrap()).unwrap();
    assert_eq!(back, k);
}

#[test]
fn a_newer_copy_keeps_when_it_was_first_seen() {
    let mut k = kept();
    let mut newer = kept();
    newer.likes = 9;
    newer.seen = "2026-04-01T00:00:00Z".into();
    k.refresh(newer);
    assert_eq!(k.likes, 9);
    assert_eq!(k.seen, "2026-03-05T00:00:00Z");
}

#[test]
fn parses_the_search_language() {
    let q = Query::parse(
        r#"rust "rainy day" from:@Alice.test since:2026-01-02 domain:www.Example.com is:liked bogus:x"#,
    );
    assert_eq!(q.words, ["rust", "rainy day", "bogus:x"]);
    assert_eq!(q.from.as_deref(), Some("alice.test"));
    assert_eq!(q.since.as_deref(), Some("2026-01-02"));
    assert_eq!(q.domain.as_deref(), Some("example.com"));
    assert!(q.liked && !q.saved);
    assert_eq!(Query::parse("since:yesterday").words, ["since:yesterday"]);
    assert!(Query::parse("   ").is_empty());
}

#[test]
fn matches_on_words_people_dates_sites_and_marks() {
    let k = kept();
    let hit = |q: &str| Query::parse(q).matches(&k);
    assert!(hit("RUST rainy"));
    assert!(hit("cats"), "the link card counts");
    assert!(!hit("rust dogs"));
    assert!(hit("from:alice.test") && hit("from:alice") && hit("from:did:plc:a"));
    assert!(!hit("from:bob"));
    assert!(hit("since:2026-03-04 until:2026-03-04"));
    assert!(!hit("since:2026-03-05"));
    assert!(hit("domain:example.com") && hit("domain:rust-lang.org"));
    assert!(!hit("domain:ample.com"));
    assert!(hit("is:liked is:saved"));
    assert!(!hit("is:mine"));
}

#[test]
fn finds_the_site_of_an_address() {
    assert_eq!(
        domain_of("https://www.Example.com:8080/x?y").as_deref(),
        Some("example.com")
    );
    assert_eq!(
        domain_of("https://u@host.test/").as_deref(),
        Some("host.test")
    );
    assert_eq!(domain_of("not a url"), None);
}
