use super::*;
use iced::Element;
use iced::widget::text;
use iced_test::simulator;

#[derive(Clone)]
struct Fake;

impl CollectionsApi for Fake {
    async fn posts(&self, _: &[AtUri]) -> Result<Vec<PostView>, ApiError> {
        Ok(Vec::new())
    }
}

fn collection(id: u64, posts: &[&str]) -> Collection {
    Collection {
        id,
        name: format!("C{id}"),
        posts: posts.iter().map(|p| AtUri::from(*p)).collect(),
    }
}

#[test]
fn a_new_collection_can_start_with_a_post() {
    let mut s = Screen::default();
    let post = AtUri::from("at://a/p/1");
    let out = s.update(Msg::NewWith(post.clone()), &[], &Fake);
    assert_eq!(out.events, [Event::Show]);
    let out = s.update(Msg::Create, &[], &Fake);
    assert!(out.events.is_empty(), "no name yet");
    let _ = s.update(Msg::Name("Recipes".into()), &[], &Fake);
    let out = s.update(Msg::Create, &[], &Fake);
    assert_eq!(out.events, [Event::Create("Recipes".into(), Some(post))]);
    let out = s.update(Msg::Name("Art".into()), &[], &Fake);
    assert!(out.events.is_empty());
    let out = s.update(Msg::Create, &[], &Fake);
    assert_eq!(out.events, [Event::Create("Art".into(), None)]);
}

#[test]
fn an_empty_collection_loads_without_asking_bluesky() {
    let mut s = Screen::default();
    let all = [collection(1, &[])];
    let out = s.update(Msg::Load(1), &all, &Fake);
    assert_eq!(out.events, [Event::Posts(Vec::new())]);
    assert!(matches!(s.posts(1), Some(Ok(p)) if p.is_empty()));
    let _ = s.update(
        Msg::Loaded(2, Err(ApiError::Other("offline".into()))),
        &all,
        &Fake,
    );
    assert!(matches!(s.posts(2), Some(Err(_))));
}

struct Plain;

impl Host<Msg> for Plain {
    fn post<'a>(&'a self, p: &'a PostView) -> Element<'a, Msg> {
        text(p.record.text.as_str()).into()
    }
}

#[test]
fn the_list_opens_and_deletes() {
    let s = Screen::default();
    let all = [collection(1, &["at://a/p/1"])];
    let mut ui = simulator(
        List {
            screen: &s,
            collections: &all,
        }
        .view(),
    );
    assert!(ui.find("1 post").is_ok());
    ui.click("Open").unwrap();
    ui.click("Delete").unwrap();
    let msgs: Vec<Msg> = ui.into_messages().collect();
    assert!(matches!(&msgs[..], [Msg::Open(1), Msg::Delete(1)]));

    let mut ui = simulator(
        Page {
            screen: &s,
            collection: &all[0],
            host: &Plain,
            lift: |m| m,
        }
        .view(),
    );
    assert!(ui.find("Loading…").is_ok());
}
