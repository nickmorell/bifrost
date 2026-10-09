//! Search: suggestions while typing, running and paging searches, dropping stale
//! answers, the Explore page and trends, and the search screen.

mod fake;

use super::*;
use crate::api::ApiError;
use crate::features;
use crate::model::{Author, FeedGenerator, FeedSource, Page, PostView, Trend};
use fake::Fake;
use iced::Element;
use iced::widget::text;
use iced_test::simulator;
use serde_json::json;

fn run(s: &mut Search, msg: Msg) -> features::Update<Msg, Event> {
    s.update(msg, &Fake)
}

fn events(s: &mut Search, msg: Msg) -> Vec<Event> {
    run(s, msg).events
}

fn author(name: &str) -> Author {
    serde_json::from_value(json!({
        "did": format!("did:plc:{name}"),
        "handle": format!("{name}.test"),
        "avatar": format!("https://a/{name}.jpg"),
    }))
    .unwrap()
}

fn post(uri: &str, text: &str) -> PostView {
    serde_json::from_value(json!({
        "uri": uri, "cid": "c",
        "author": {"did": "did:plc:ann", "handle": "ann.test"},
        "record": {"text": text, "createdAt": "2026-10-04T02:00:00Z"},
    }))
    .unwrap()
}

fn trend(topic: &str, link: &str) -> Trend {
    serde_json::from_value(json!({"topic": topic, "link": link})).unwrap()
}

fn page<T>(items: Vec<T>, cursor: Option<&str>) -> Page<T> {
    Page {
        items,
        cursor: cursor.map(Into::into),
    }
}

/// Types `q` and submits it.
fn searched(s: &mut Search, q: &str) -> features::Update<Msg, Event> {
    let _ = run(s, Msg::Input(q.into()));
    run(s, Msg::Submit)
}

#[test]
fn typing_suggests_people_after_a_pause_but_not_for_tags() {
    let mut s = Search::default();
    assert_eq!(run(&mut s, Msg::Input("an".into())).task.units(), 1);
    let first = s.suggest_seq;
    assert_eq!(run(&mut s, Msg::Input("ann".into())).task.units(), 1);
    assert_eq!(
        run(&mut s, Msg::Suggest(first)).task.units(),
        0,
        "typing went on, so the older pause asks for nothing"
    );
    let latest = s.suggest_seq;
    assert_eq!(run(&mut s, Msg::Suggest(latest)).task.units(), 1);

    let ev = events(&mut s, Msg::Suggested(first, Ok(vec![author("old")])));
    assert!(
        ev.is_empty() && s.suggest.is_empty(),
        "stale suggestions drop"
    );
    let ev = events(&mut s, Msg::Suggested(latest, Ok(vec![author("ann")])));
    assert!(matches!(&ev[..], [Event::Pictures(u)] if u == &["https://a/ann.jpg"]));
    assert_eq!(s.suggest.len(), 1);

    assert_eq!(run(&mut s, Msg::Input("#ru".into())).task.units(), 0);
    assert!(s.suggest.is_empty());
}

#[test]
fn submitting_searches_the_open_tab_and_remembers_the_query() {
    let mut s = Search::default();
    let out = searched(&mut s, "  rust  ");
    assert_eq!(out.task.units(), 1);
    assert!(matches!(&out.events[..], [Event::Remember(q)] if q == "rust"));
    assert!(s.searched() && !s.typing);
    assert_eq!(s.tab, Some(Tab::Top));
    assert!(s.loading(Tab::Top));

    let _ = run(&mut s, Msg::Input("   ".into()));
    let out = run(&mut s, Msg::Submit);
    assert!(
        out.events.is_empty() && out.task.units() == 0,
        "blank is ignored"
    );
}

#[test]
fn tag_searches_only_have_posts() {
    let mut s = Search::default();
    let _ = searched(&mut s, "rust");
    let _ = run(&mut s, Msg::Tab(Tab::People));
    let _ = searched(&mut s, "#rust");
    assert_eq!(s.tag(), Some("rust"));
    assert_eq!(s.tab, Some(Tab::Top));
    let _ = searched(&mut s, "#two words");
    assert_eq!(s.tag(), None);
}

#[test]
fn answers_to_an_older_search_are_dropped() {
    let mut s = Search::default();
    let _ = searched(&mut s, "cats");
    let old = s.seq;
    let _ = searched(&mut s, "dogs");
    let ev = events(
        &mut s,
        Msg::Posts(
            old,
            Tab::Top,
            false,
            Ok(page(vec![post("at://p/1", "cat")], None)),
        ),
    );
    assert!(ev.is_empty() && s.posts().is_empty());

    let seq = s.seq;
    let ev = events(
        &mut s,
        Msg::Posts(
            seq,
            Tab::Top,
            false,
            Ok(page(vec![post("at://p/2", "dog")], None)),
        ),
    );
    assert!(matches!(&ev[..], [Event::Posts(u)] if u.len() == 1 && u[0].as_str() == "at://p/2"));
    assert!(s.post("at://p/2").is_some() && !s.loading(Tab::Top));

    let ev = events(
        &mut s,
        Msg::People(seq, false, Err(ApiError::Other("down".into()))),
    );
    assert!(matches!(&ev[..], [Event::Failed(_)]));
}

#[test]
fn more_loads_the_next_page_only_when_there_is_one() {
    let mut s = Search::default();
    let _ = searched(&mut s, "cats");
    assert_eq!(run(&mut s, Msg::More).task.units(), 0, "still loading");
    let seq = s.seq;
    let _ = run(
        &mut s,
        Msg::Posts(
            seq,
            Tab::Top,
            false,
            Ok(page(vec![post("at://p/1", "a")], Some("c2"))),
        ),
    );
    assert_eq!(run(&mut s, Msg::More).task.units(), 1);
    let _ = run(
        &mut s,
        Msg::Posts(
            seq,
            Tab::Top,
            true,
            Ok(page(vec![post("at://p/2", "b")], None)),
        ),
    );
    assert_eq!(s.posts().len(), 2, "the next page is added");
    assert_eq!(run(&mut s, Msg::More).task.units(), 0, "no more pages");
}

#[test]
fn a_tab_loads_once_when_first_opened() {
    let mut s = Search::default();
    let _ = searched(&mut s, "cats");
    assert_eq!(run(&mut s, Msg::Tab(Tab::People)).task.units(), 1);
    assert_eq!(run(&mut s, Msg::Tab(Tab::People)).task.units(), 0);
    let seq = s.seq;
    let _ = run(
        &mut s,
        Msg::People(seq, false, Ok(page(vec![author("ann")], None))),
    );
    assert_eq!(run(&mut s, Msg::Tab(Tab::People)).task.units(), 0);
    let _ = run(
        &mut s,
        Msg::Posts(seq, Tab::Top, false, Ok(page(vec![], None))),
    );
    assert_eq!(run(&mut s, Msg::Tab(Tab::Top)).task.units(), 0);
    assert_eq!(
        run(&mut s, Msg::Tab(Tab::Latest)).task.units(),
        1,
        "Top's posts are not Latest's"
    );
}

#[test]
fn searching_for_something_opens_the_screen_and_clearing_goes_back_to_explore() {
    let mut s = Search::default();
    let ev = events(&mut s, Msg::For("rust".into()));
    assert!(matches!(&ev[..], [Event::Show, Event::Remember(q)] if q == "rust"));
    let ev = events(&mut s, Msg::Clear);
    assert!(matches!(&ev[..], [Event::Explore(false)]));
    assert!(!s.searched() && s.query.is_empty());
}

#[test]
fn explore_loads_only_what_is_missing() {
    let mut s = Search::default();
    assert_eq!(run(&mut s, Msg::Explore { force: false }).task.units(), 2);
    let _ = run(&mut s, Msg::Trends(Err(ApiError::Other("down".into()))));
    let _ = run(&mut s, Msg::SuggestedPeople(Ok(vec![author("ann")])));
    assert_eq!(
        run(&mut s, Msg::Explore { force: false }).task.units(),
        1,
        "failed trends are tried again"
    );
    let _ = run(
        &mut s,
        Msg::Trends(Ok(vec![trend("Rust", "/search?q=rust")])),
    );
    assert_eq!(run(&mut s, Msg::Explore { force: false }).task.units(), 0);
    assert_eq!(run(&mut s, Msg::Explore { force: true }).task.units(), 2);
}

#[test]
fn trends_search_or_open_their_feed() {
    let mut s = Search::default();
    let ev = events(&mut s, Msg::OpenTrend(trend("Rust", "/search?q=rustlang")));
    assert!(matches!(&ev[..], [Event::Show, Event::Remember(q)] if q == "rustlang"));

    let feed = trend("Cats", "/profile/cats.test/feed/aaa");
    assert_eq!(run(&mut s, Msg::OpenTrend(feed)).task.units(), 1);
    let uri = "at://did:plc:c/app.bsky.feed.generator/aaa";
    let ev = events(&mut s, Msg::TrendFeed("Cats".into(), Ok(uri.into())));
    assert!(
        matches!(&ev[..], [Event::OpenFeed(FeedSource::Generator(u), n)] if u.as_str() == uri && n == "Cats")
    );
    let ev = events(
        &mut s,
        Msg::TrendFeed("Cats".into(), Err(ApiError::Other("gone".into()))),
    );
    assert!(
        matches!(&ev[..], [Event::Show, Event::Remember(q)] if q == "Cats"),
        "a feed that can't be found is searched for instead"
    );
}

/// The parts of the search screen the app draws, as plain stand-ins.
struct Plain;

impl Host<Msg> for Plain {
    fn post<'a>(&'a self, post: &'a PostView) -> Element<'a, Msg> {
        text(post.record.text.as_str()).into()
    }
    fn person<'a>(&'a self, author: &'a Author) -> Element<'a, Msg> {
        text(format!("@{}", author.handle)).into()
    }
    fn feed<'a>(&'a self, feed: &'a FeedGenerator) -> Element<'a, Msg> {
        text(&feed.display_name).into()
    }
    fn retry<'a>(&self, why: String) -> Element<'a, Msg> {
        text(why).into()
    }
    fn saved<'a>(&'a self, query: &'a str) -> Element<'a, Msg> {
        text(format!("kept posts for {query}")).into()
    }
    fn top_links(&self) -> Option<Element<'_, Msg>> {
        Some(text("the links").into())
    }
    fn scroll<'a>(&self, content: Element<'a, Msg>) -> Element<'a, Msg> {
        content
    }
}

fn screen<'a>(s: &'a Search, recent: &'a [String], muted_tag: Option<&'a str>) -> Element<'a, Msg> {
    View {
        search: s,
        recent,
        tags: &[],
        muted_tag,
        feeds: &[],
        host: &Plain,
        lift: |m| m,
    }
    .screen()
}

#[test]
fn explore_shows_recent_searches_and_trends() {
    let mut s = Search::default();
    let _ = run(
        &mut s,
        Msg::Trends(Ok(vec![trend("Rust", "/search?q=rust")])),
    );
    let _ = run(&mut s, Msg::SuggestedPeople(Ok(vec![author("ann")])));
    let recent = vec!["cats".to_string()];
    let mut ui = simulator(screen(&s, &recent, None));
    assert!(ui.find("@ann.test").is_ok());
    assert!(ui.find("From posts you saw").is_ok() && ui.find("the links").is_ok());
    ui.click("cats").unwrap();
    ui.click("Clear all").unwrap();
    ui.click("Rust").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::For(q), Msg::Forget(None), Msg::OpenTrend(t)] if q == "cats" && t.topic == "Rust"
    ));
}

#[test]
fn typing_offers_to_search_and_matching_recent_searches() {
    let mut s = Search::default();
    let _ = run(&mut s, Msg::Input("ca".into()));
    let recent = vec!["cats".to_string(), "dogs".to_string()];
    let mut ui = simulator(screen(&s, &recent, None));
    assert!(ui.find("dogs").is_err());
    ui.click("Search for \u{201c}ca\u{201d}").unwrap();
    ui.click("cats").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::For(a), Msg::For(b)] if a == "ca" && b == "cats"
    ));
}

#[test]
fn results_have_tabs_and_tag_pages_offer_muting() {
    let mut s = Search::default();
    let _ = searched(&mut s, "cats");
    let seq = s.seq;
    let _ = run(
        &mut s,
        Msg::Posts(
            seq,
            Tab::Top,
            false,
            Ok(page(vec![post("at://p/1", "a cat")], None)),
        ),
    );
    let mut ui = simulator(screen(&s, &[], None));
    assert!(ui.find("a cat").is_ok());
    ui.click("People").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Tab(Tab::People)]
    ));

    let _ = run(&mut s, Msg::Tab(Tab::Saved));
    let mut ui = simulator(screen(&s, &[], None));
    assert!(ui.find("kept posts for cats").is_ok());
    drop(ui);
    assert!(!s.loading(Tab::Saved), "searched here, not on Bluesky");

    let _ = searched(&mut s, "#Rust");
    let mut ui = simulator(screen(&s, &[], Some("rust")));
    assert!(ui.find("People").is_err(), "tag pages only have posts");
    ui.click("Unmute tag").unwrap();
    ui.click("Post with tag").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::MuteTag(w, false), Msg::PostWithTag(t)] if w == "rust" && t == "Rust"
    ));
}
