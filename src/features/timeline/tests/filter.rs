//! Feed filters: what they leave out, and loading on when too little is left.

use super::*;
use crate::model::FeedFilter;
use std::collections::BTreeMap;

fn run_filtered(t: &mut Timelines, filter: FeedFilter, msg: Msg) -> features::Update<Msg, Event> {
    let prefs = FollowingPrefs::default();
    let filters = BTreeMap::from([("following".to_string(), filter)]);
    let ctx = Ctx {
        following: &prefs,
        me: "did:plc:me",
        filters: &filters,
    };
    t.update(msg, &ctx, &Fake)
}

#[test]
fn a_feed_filter_leaves_posts_out_and_loads_on_when_little_is_left() {
    let mut t = Timelines::default();
    let hide = FeedFilter {
        hide_reposts: true,
        ..FeedFilter::default()
    };
    let _ = run_filtered(
        &mut t,
        hide,
        Msg::Load {
            key: HOME,
            force: false,
        },
    );
    let out = run_filtered(
        &mut t,
        hide,
        Msg::Loaded(
            HOME,
            Load::First,
            Ok(page(vec![item(3), repost(2), item(1)], Some("c"))),
        ),
    );
    assert_eq!(shown(&t, &HOME), uris(&[item(3), item(1)]));
    // Two posts can't fill the screen, so the next page is asked for.
    assert!(matches!(&out.events[..], [Event::Fetched(_), Event::Empty(k)] if *k == HOME));

    // Other feeds are not touched.
    let other = Key::Bookmarks;
    let _ = run_filtered(
        &mut t,
        hide,
        Msg::Loaded(other.clone(), Load::First, Ok(page(vec![repost(2)], None))),
    );
    assert_eq!(shown(&t, &other), uris(&[repost(2)]));
}
