//! The ATstore feature against a fake directory.

use super::*;
use crate::components::Picture;
use crate::features::Pictures;
use iced::Element;
use iced_test::simulator;
use serde_json::json;

const NOW: &str = "2026-10-14T12:00:00Z";

/// Answers every call with "not in the fake": the tests hand the feature its answers
/// as messages themselves.
#[derive(Clone)]
struct Fake;

impl crate::api::service::AtstoreApi for Fake {
    fn catalog(&self) -> impl Future<Output = Result<Catalog, ApiError>> + Send {
        std::future::ready(Err(ApiError::Other("catalog is not in the fake".into())))
    }
    fn listing(&self, uri: &AtUri) -> impl Future<Output = Result<ListingDetail, ApiError>> + Send {
        std::future::ready(Err(ApiError::Other(format!("{uri} is not in the fake"))))
    }
}

struct NoPictures;

impl Pictures for NoPictures {
    fn picture(&self, _: Option<&str>) -> Option<Picture> {
        None
    }
}

fn listing(name: &str, tags: &[&str], rating: &str, reviews: u64) -> Listing {
    serde_json::from_value(json!({
        "uri": format!("at://did:plc:a/fyi.atstore.listing.detail/{name}"),
        "name": name, "tagline": format!("{name} does things"),
        "iconUrl": format!("https://cdn.example/{name}.png"),
        "heroImageUrl": format!("https://cdn.example/{name}-hero.png"),
        "appTags": tags, "rating": rating, "reviewCount": reviews,
        "productAccountHandle": format!("{}.example", name.to_lowercase()),
    }))
    .unwrap()
}

fn catalog(fetched: &str) -> Catalog {
    Catalog {
        fetched: fetched.into(),
        listings: vec![
            listing("Witchsky", &["social", "bluesky client"], "4.8", 26),
            listing("Graze", &["social"], "5", 9),
            listing("Arabica", &["food"], "0", 0),
        ],
        ..Catalog::default()
    }
}

fn open(offline: bool, data_saver: bool) -> Msg {
    Msg::Open {
        now: NOW.into(),
        offline,
        data_saver,
    }
}

fn names(s: &Store) -> Vec<&str> {
    s.matching().iter().map(|l| l.name.as_str()).collect()
}

fn uri(name: &str) -> AtUri {
    format!("at://did:plc:a/fyi.atstore.listing.detail/{name}")
        .as_str()
        .into()
}

#[test]
fn opening_reads_the_directory_unless_the_kept_one_will_do() {
    let mut s = Store::default();
    let _ = s.update(open(false, false), &Fake);
    assert!(s.loading, "nothing kept yet");

    for (offline, data_saver) in [(false, false), (true, false), (false, true)] {
        let mut s = Store::default();
        s.restore(Some(catalog("2026-10-14T11:00:00Z")));
        let _ = s.update(open(offline, data_saver), &Fake);
        assert!(!s.loading, "fresh enough");
    }

    let mut s = Store::default();
    s.restore(Some(catalog("2026-10-13T00:00:00Z")));
    let _ = s.update(open(false, false), &Fake);
    assert!(s.loading, "stale");
    let mut s = Store::default();
    s.restore(Some(catalog("2026-10-13T00:00:00Z")));
    let _ = s.update(open(true, false), &Fake);
    assert!(!s.loading, "stale, but offline");
    let _ = s.update(open(false, true), &Fake);
    assert!(!s.loading, "stale, but the data saver is on");
    let _ = s.update(Msg::Refresh, &Fake);
    assert!(s.loading, "asked to refresh");
}

#[test]
fn a_read_directory_is_kept_and_a_failed_read_keeps_the_old_one() {
    let mut s = Store::default();
    s.restore(Some(catalog("2026-10-01T00:00:00Z")));
    let _ = s.update(Msg::Refresh, &Fake);
    let out = s.update(Msg::Loaded(Err(ApiError::Other("down".into()))), &Fake);
    assert!(out.events.is_empty());
    assert_eq!(s.error.as_deref(), Some("down"));
    assert_eq!(names(&s).len(), 3, "still shows what was kept");

    let mut newer = catalog(NOW);
    newer.listings.truncate(1);
    let out = s.update(Msg::Loaded(Ok(Box::new(newer))), &Fake);
    assert_eq!(s.error, None);
    assert_eq!(names(&s), ["Witchsky"]);
    assert!(matches!(&out.events[0], Event::Save(c) if c.fetched == NOW));
    assert!(
        matches!(&out.events[1], Event::Pictures(u) if u == &["https://cdn.example/Witchsky.png"])
    );
}

#[test]
fn restoring_never_replaces_a_directory_already_read() {
    let mut s = Store::default();
    s.restore(Some(catalog(NOW)));
    s.restore(Some(Catalog::default()));
    assert_eq!(s.catalog.fetched, NOW);
    s.restore(None);
    assert!(s.has_catalog());
}

#[test]
fn search_tags_and_order_pick_what_is_shown() {
    let mut s = Store::default();
    s.restore(Some(catalog(NOW)));
    assert_eq!(names(&s), ["Witchsky", "Graze", "Arabica"]);
    let _ = s.update(Msg::Sort(Sort::Rating), &Fake);
    assert_eq!(names(&s), ["Graze", "Witchsky", "Arabica"], "unrated last");
    let _ = s.update(Msg::Sort(Sort::Name), &Fake);
    assert_eq!(names(&s), ["Arabica", "Graze", "Witchsky"]);
    let _ = s.update(Msg::Tag(Some("social".into())), &Fake);
    assert_eq!(names(&s), ["Graze", "Witchsky"]);
    let _ = s.update(Msg::Query("witch".into()), &Fake);
    assert_eq!(names(&s), ["Witchsky"]);
    let _ = s.update(Msg::Tag(None), &Fake);
    let _ = s.update(Msg::Query("does".into()), &Fake);
    assert_eq!(names(&s).len(), 3);
}

#[test]
fn only_the_shown_listings_ask_for_their_icons() {
    let mut s = Store::default();
    let mut c = catalog(NOW);
    let one = c.listings[0].clone();
    c.listings = (0..PAGE + 5)
        .map(|i| Listing {
            uri: uri(&i.to_string()),
            icon_url: Some(format!("https://cdn.example/{i}.png")),
            ..one.clone()
        })
        .collect();
    s.restore(Some(c));
    let out = s.update(open(false, false), &Fake);
    assert!(matches!(&out.events[0], Event::Pictures(u) if u.len() == PAGE));
    let out = s.update(Msg::More, &Fake);
    assert!(matches!(&out.events[0], Event::Pictures(u) if u.len() == PAGE + 5));
}

#[test]
fn a_listing_page_is_read_once_and_kept() {
    let mut s = Store::default();
    s.restore(Some(catalog(NOW)));
    let w = uri("Witchsky");
    let out = s.update(Msg::Select(w.clone()), &Fake);
    assert!(matches!(&out.events[0], Event::OpenListing(u) if u == &w));

    let _ = s.update(
        Msg::Show {
            uri: w.clone(),
            offline: false,
        },
        &Fake,
    );
    assert_eq!(s.pages.get(&w), Some(&Reading::Loading));

    let detail = ListingDetail {
        listing: Listing {
            tagline: "Newer words".into(),
            ..s.catalog.listings[0].clone()
        },
        screenshots: vec!["https://cdn.example/shot.png".into()],
        external_url: Some("https://witchsky.app/".into()),
        links: Vec::new(),
    };
    let out = s.update(Msg::Page(w.clone(), Ok(Box::new(detail))), &Fake);
    assert!(matches!(&out.events[0], Event::Save(c) if c.details.contains_key(&w)));
    assert!(matches!(&out.events[1], Event::Pictures(u) if u.len() == 3));
    assert_eq!(s.catalog.listings[0].tagline, "Newer words");
    assert_eq!(s.pages.get(&w), None);

    let out = s.update(
        Msg::Show {
            uri: w.clone(),
            offline: false,
        },
        &Fake,
    );
    assert!(
        matches!(&out.events[0], Event::Pictures(_)),
        "kept, not read again"
    );
    assert_eq!(s.pages.get(&w), None);
}

#[test]
fn offline_a_listing_shows_what_the_directory_has() {
    let mut s = Store::default();
    s.restore(Some(catalog(NOW)));
    let g = uri("Graze");
    let _ = s.update(
        Msg::Show {
            uri: g.clone(),
            offline: true,
        },
        &Fake,
    );
    assert_eq!(s.pages.get(&g), None);
    let _ = s.update(Msg::Reload(g.clone()), &Fake);
    let _ = s.update(
        Msg::Page(g.clone(), Err(ApiError::Other("down".into()))),
        &Fake,
    );
    assert_eq!(s.pages.get(&g), Some(&Reading::Failed("down".into())));
    assert_eq!(s.title(&g), "Graze");
    assert_eq!(s.title(&uri("Nope")), "ATstore");
}

#[test]
fn links_ask_the_app_to_open_them() {
    let mut s = Store::default();
    let out = s.update(Msg::OpenUrl("https://witchsky.app/".into()), &Fake);
    assert!(matches!(&out.events[0], Event::OpenUrl(u) if u == "https://witchsky.app/"));
    let out = s.update(Msg::OpenProfile("graze.social".into()), &Fake);
    assert!(matches!(&out.events[0], Event::OpenProfile(h) if h == "graze.social"));
}

fn view(s: &Store, data_saver: bool) -> Element<'_, Msg> {
    View {
        store: s,
        pictures: &NoPictures,
        data_saver,
    }
    .browse()
}

#[test]
fn views_draw_in_both_themes() {
    let mut s = Store::default();
    let _ = s.update(open(false, false), &Fake);
    assert!(simulator(view(&s, false)).find("Loading ATstore…").is_ok());

    s.restore(Some(catalog(NOW)));
    {
        let mut ui = simulator(view(&s, false));
        assert!(ui.find("Witchsky").is_ok());
        assert!(ui.find("Rated 4.8 · 26 reviews").is_ok());
        assert!(ui.find("Bluesky client").is_ok(), "tags offered");
    }
    assert!(
        simulator(view(&s, false))
            .snapshot(&iced::Theme::Light)
            .is_ok()
    );

    let w = uri("Witchsky");
    let detail = ListingDetail {
        listing: s.catalog.listings[0].clone(),
        screenshots: vec!["https://cdn.example/shot.png".into()],
        external_url: Some("https://www.witchsky.app/".into()),
        links: vec![crate::model::ListingLink {
            uri: "https://witchsky.app/privacy".into(),
        }],
    };
    let _ = s.update(Msg::Page(w.clone(), Ok(Box::new(detail))), &Fake);
    let page = View {
        store: &s,
        pictures: &NoPictures,
        data_saver: true,
    };
    let mut ui = simulator(page.listing(&w));
    assert!(ui.find("Open witchsky.app").is_ok());
    assert!(ui.find("@witchsky.example").is_ok());
    assert!(ui.find("witchsky.app/privacy").is_ok());
    assert!(
        simulator(page.listing(&w))
            .snapshot(&iced::Theme::Dark)
            .is_ok()
    );
}
