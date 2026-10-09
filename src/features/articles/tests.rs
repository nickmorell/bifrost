use super::*;
use crate::model::{Block, DOCUMENT, PUBLICATION, SUBSCRIPTION};

#[derive(Clone, Default)]
struct Fake;

impl ArticlesApi for Fake {
    async fn article(&self, _: &ArticleRef) -> Result<Article, ApiError> {
        Err(ApiError::Other("offline".into()))
    }
    async fn subscriptions(&self) -> Result<Vec<(AtUri, AtUri)>, ApiError> {
        Ok(Vec::new())
    }
    async fn subscribe(&self, _: &AtUri) -> Result<AtUri, ApiError> {
        Ok(record())
    }
    async fn unsubscribe(&self, _: &AtUri) -> Result<(), ApiError> {
        Ok(())
    }
    async fn latest(&self, _: &[AtUri]) -> Vec<ArticleSummary> {
        Vec::new()
    }
}

fn publication() -> AtUri {
    AtUri::new("did:plc:a", PUBLICATION, "self")
}

fn record() -> AtUri {
    AtUri::new("did:plc:me", SUBSCRIPTION, "1")
}

fn article() -> Article {
    Article {
        title: "Hello".into(),
        description: String::new(),
        published: String::new(),
        url: None,
        cover: Some("https://pds.example/cover".into()),
        blocks: vec![Block::Paragraph("words".into())],
        publication: None,
    }
}

#[test]
fn subscribing_waits_for_the_server_and_can_be_undone() {
    let api = Fake;
    let mut a = Articles::default();
    let _ = a.update(Msg::Toggle(publication()), &api);
    assert_eq!(a.subscription(&publication()), (false, true));
    // A second press while it is changing does nothing.
    let _ = a.update(Msg::Toggle(publication()), &api);

    let out = a.update(Msg::Subscribed(publication(), Ok(record())), &api);
    assert_eq!(out.events, [Event::Toast("Subscribed.".into())]);
    assert_eq!(a.subscription(&publication()), (true, false));

    let _ = a.update(Msg::Toggle(publication()), &api);
    assert_eq!(a.subscription(&publication()), (false, true));
    let out = a.update(
        Msg::Unsubscribed(publication(), record(), Err(ApiError::Other("no".into()))),
        &api,
    );
    assert!(matches!(&out.events[..], [Event::Toast(t)] if t.starts_with("Couldn't unsubscribe")));
    assert_eq!(a.subscription(&publication()), (true, false), "put back");
}

#[test]
fn loaded_subscriptions_are_keyed_by_publication() {
    let mut a = Articles::default();
    let _ = a.update(Msg::Loaded(Ok(vec![(record(), publication())])), &Fake);
    assert_eq!(a.subscription(&publication()), (true, false));
    assert!(a.stale());
}

#[test]
fn the_reader_shows_only_the_article_last_opened() {
    let api = Fake;
    let mut a = Articles::default();
    let first = ArticleRef::Standard {
        uri: AtUri::new("did:plc:a", DOCUMENT, "1"),
        link: None,
    };
    let second = ArticleRef::Standard {
        uri: AtUri::new("did:plc:a", DOCUMENT, "2"),
        link: None,
    };
    let _ = a.update(Msg::Open(first.clone()), &api);
    let _ = a.update(Msg::Open(second.clone()), &api);
    let out = a.update(Msg::Opened(first, Ok(article())), &api);
    assert!(out.events.is_empty());
    assert!(matches!(a.reader, Some((_, Reading::Loading))));

    let out = a.update(Msg::Opened(second.clone(), Ok(article())), &api);
    assert_eq!(
        out.events,
        [Event::Pictures(vec!["https://pds.example/cover".into()])]
    );
    // Opening the same article again keeps it.
    let _ = a.update(Msg::Open(second), &api);
    assert!(matches!(a.reader, Some((_, Reading::Ready(_)))));
}

#[test]
fn rows_go_to_the_article() {
    let a = ArticleRef::Standard {
        uri: AtUri::new("did:plc:a", DOCUMENT, "1"),
        link: None,
    };
    let out = Articles::default().update(Msg::Read(a.clone()), &Fake);
    assert_eq!(out.events, [Event::Read(a)]);
}
