//! Keeping your place: finding the post you were reading as a timeline loads, and
//! which post is at the top of a scrolled timeline.

use super::*;

#[test]
fn the_post_you_were_reading_is_found_and_marked() {
    let mut t = Timelines::default();
    let want = item_key(&item(2));
    // Asked for before the first page arrives.
    let out = run(&mut t, Msg::Find(HOME, want.clone()));
    assert_eq!(out.task.units(), 0);
    let _ = run(
        &mut t,
        Msg::Load {
            key: HOME,
            force: false,
        },
    );
    let out = run(
        &mut t,
        Msg::Loaded(
            HOME,
            Load::First,
            Ok(page(vec![item(4), item(3), item(2), item(1)], Some("c"))),
        ),
    );
    assert!(
        out.events
            .iter()
            .any(|e| matches!(e, Event::Found(k, i) if *k == HOME && *i == want))
    );
    let tl = t.get(&HOME).unwrap();
    assert_eq!(tl.marker(), Some(want.as_str()));
    // Two posts above it, drawn at the guessed height until measured.
    assert_eq!(tl.top_of(&want), Some(480.0));
    let _ = run(
        &mut t,
        Msg::Measured(
            HOME,
            crate::components::row_key(item_key(&item(4)).as_str()),
            100.0,
        ),
    );
    let tl = t.get(&HOME).unwrap();
    assert_eq!(tl.top_of(&want), Some(340.0));
    assert_eq!(tl.item_at(0.0), Some(item_key(&item(4))));
    assert_eq!(tl.item_at(99.0), Some(item_key(&item(4))));
    assert_eq!(tl.item_at(100.0), Some(item_key(&item(3))));
}

#[test]
fn looking_goes_a_few_pages_deep_then_gives_up() {
    let mut t = Timelines::default();
    let _ = run(&mut t, Msg::Find(HOME, "gone".into()));
    let _ = run(
        &mut t,
        Msg::Load {
            key: HOME,
            force: false,
        },
    );
    let out = run(
        &mut t,
        Msg::Loaded(HOME, Load::First, Ok(page(vec![item(9)], Some("c1")))),
    );
    assert_eq!(out.task.units(), 1, "the next page is fetched to look in");
    let mut n = 8;
    let mut fetched = 1;
    while fetched < 10 {
        let out = run(
            &mut t,
            Msg::Loaded(HOME, Load::More, Ok(page(vec![item(n)], Some("c")))),
        );
        n -= 1;
        if out.task.units() == 0 {
            break;
        }
        fetched += 1;
    }
    assert!(fetched < 10, "it stops");
    assert!(t.get(&HOME).unwrap().marker().is_none());
}

#[test]
fn the_top_post_is_not_marked() {
    let mut t = Timelines::default();
    let want = item_key(&item(2));
    loaded(&mut t, &HOME, vec![item(2), item(1)], None);
    let out = run(&mut t, Msg::Find(HOME, want.clone()));
    assert!(matches!(&out.events[..], [Event::Found(_, i)] if *i == want));
    assert!(t.get(&HOME).unwrap().marker().is_none());
}
