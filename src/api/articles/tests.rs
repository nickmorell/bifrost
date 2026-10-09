use super::*;
use crate::net::fake::Fake;

const DID_DOC: &str =
    r##"{"service":[{"id":"#atproto_pds","serviceEndpoint":"https://pds.example"}]}"##;

fn record(value: Value) -> String {
    json!({"uri": "at://did:plc:a/x/y", "cid": "c", "value": value}).to_string()
}

fn publication_record() -> String {
    record(json!({
        "name": "Field Notes",
        "url": "https://notes.example/",
        "icon": {"ref": {"$link": "bafyicon"}}
    }))
}

#[tokio::test]
async fn reads_a_standard_article_with_its_publication() {
    let net = Fake::default()
        .reply(200, DID_DOC)
        .reply(
            200,
            &record(json!({
                "site": "at://did:plc:a/site.standard.publication/self",
                "path": "/posts/hello",
                "title": "Hello",
                "description": "A first post",
                "publishedAt": "2026-10-01T10:00:00Z",
                "textContent": "# Start\n\nSome words here."
            })),
        )
        .reply(200, DID_DOC)
        .reply(200, &publication_record());
    let uri = AtUri::new("did:plc:a", DOCUMENT, "3k1");
    let a = Reader(&net).standard(&uri, None).await.unwrap();
    assert_eq!(a.title, "Hello");
    assert_eq!(a.url.as_deref(), Some("https://notes.example/posts/hello"));
    assert_eq!(
        a.blocks,
        [
            Block::Heading("Start".into()),
            Block::Paragraph("Some words here.".into())
        ]
    );
    let p = a.publication.unwrap();
    assert_eq!(p.name, "Field Notes");
    assert_eq!(
        p.icon.as_deref(),
        Some("https://pds.example/xrpc/com.atproto.sync.getBlob?did=did:plc:a&cid=bafyicon")
    );
    assert_eq!(
        net.seen()[1].url.as_str(),
        "https://pds.example/xrpc/com.atproto.repo.getRecord?repo=did%3Aplc%3Aa&collection=site.standard.document&rkey=3k1"
    );
}

#[tokio::test]
async fn leaflet_content_stands_in_for_missing_text() {
    let net = Fake::default().reply(200, DID_DOC).reply(
        200,
        &record(json!({
            "site": "https://loose.example",
            "title": "Loose",
            "content": {"pages": [{"blocks": [
                {"block": {"$type": "pub.leaflet.blocks.header", "plaintext": "Top"}},
                {"block": {"$type": "pub.leaflet.blocks.text", "plaintext": "Body"}},
                {"block": {"$type": "pub.leaflet.blocks.text", "plaintext": "  "}}
            ]}]}
        })),
    );
    let a = Reader(&net)
        .standard(&AtUri::new("did:plc:a", DOCUMENT, "k"), None)
        .await
        .unwrap();
    assert_eq!(
        a.blocks,
        [
            Block::Heading("Top".into()),
            Block::Paragraph("Body".into())
        ]
    );
    assert_eq!(a.url.as_deref(), Some("https://loose.example"));
    assert_eq!(a.publication, None);
}

#[tokio::test]
async fn an_article_that_moved_is_found_by_its_web_address() {
    let gone = r#"{"error":"InvalidRequest","message":"Could not locate record"}"#;
    let net = Fake::default()
        .reply(200, DID_DOC)
        .reply(400, gone)
        .reply(
            200,
            &json!({"records": [
                {"uri": "at://did:plc:a/site.standard.document/old", "cid": "c",
                 "value": {"site": "https://loose.example", "path": "/other", "title": "No"}},
                {"uri": "at://did:plc:a/site.standard.document/new", "cid": "c",
                 "value": {"site": "https://loose.example", "path": "/posts/hi/", "title": "Yes"}}
            ]})
            .to_string(),
        )
        .reply(200, DID_DOC)
        .reply(400, gone)
        .reply(200, r#"{"records": []}"#);
    let uri = AtUri::new("did:plc:a", DOCUMENT, "gone");
    let link = Some("https://loose.example/posts/hi?ref=bsky");
    let a = Reader(&net).standard(&uri, link).await.unwrap();
    assert_eq!(a.title, "Yes");
    let missing = Reader(&net).standard(&uri, link).await.unwrap_err();
    assert!(
        missing.to_string().contains("Could not locate record"),
        "{missing}"
    );
}

#[test]
fn web_paths_ignore_queries_and_trailing_slashes() {
    assert_eq!(web_path("https://a.example/x/y/?q=1#f"), Some("x/y"));
    assert_eq!(web_path("https://a.example/"), None);
    assert_eq!(web_path("http://a.example/x"), None);
    assert_eq!(web_path_of("/x/"), Some("x"));
}

#[tokio::test]
async fn reads_whitewind_entries() {
    let net = Fake::default().reply(200, DID_DOC).reply(
        200,
        &record(json!({"title": "W", "content": "- one\n- two", "createdAt": "2026-01-01"})),
    );
    let a = Reader(&net)
        .whitewind("did:plc:a", "3kw", "alice.test")
        .await
        .unwrap();
    assert_eq!(a.url.as_deref(), Some("https://whtwnd.com/alice.test/3kw"));
    assert_eq!(a.blocks.len(), 2);
}

#[tokio::test]
async fn lists_subscriptions_and_the_newest_articles() {
    let sub = json!({"records": [
        {"uri": "at://did:plc:me/site.standard.graph.subscription/1", "cid": "c",
         "value": {"publication": "at://did:plc:a/site.standard.publication/self"}},
        {"uri": "at://did:plc:me/site.standard.graph.subscription/2", "cid": "c", "value": {}}
    ]});
    let docs = json!({"records": [
        {"uri": "at://did:plc:a/site.standard.document/1", "cid": "c", "value": {
            "site": "at://did:plc:a/site.standard.publication/self", "title": "Old",
            "publishedAt": "2026-01-01T00:00:00Z"}},
        {"uri": "at://did:plc:a/site.standard.document/2", "cid": "c", "value": {
            "site": "at://did:plc:a/site.standard.publication/other", "title": "Elsewhere",
            "publishedAt": "2026-05-01T00:00:00Z"}},
        {"uri": "at://did:plc:a/site.standard.document/3", "cid": "c", "value": {
            "site": "at://did:plc:a/site.standard.publication/self", "title": "New",
            "publishedAt": "2026-03-01T00:00:00Z"}}
    ]});
    let net = Fake::default()
        .reply(200, DID_DOC)
        .reply(200, &sub.to_string())
        .reply(200, DID_DOC)
        .reply(200, &publication_record())
        .reply(200, DID_DOC)
        .reply(200, &docs.to_string());
    let reader = Reader(&net);
    let subs = reader.subscriptions("did:plc:me").await.unwrap();
    assert_eq!(subs.len(), 1);
    let latest = reader.latest(&[subs[0].1.clone()]).await;
    let titles: Vec<&str> = latest.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, ["New", "Old"]);
    assert_eq!(latest[0].publication, "Field Notes");
}
