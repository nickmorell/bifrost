//! The stats screen against a fake Bluesky.

use super::*;
use crate::model::{Kept, Stats};
use iced::Element;
use iced::widget::text;
use iced_test::simulator;

#[derive(Clone)]
struct Fake;

impl StatsApi for Fake {
    async fn my_posts(&self, _: &str) -> Result<Vec<PostView>, ApiError> {
        Ok(Vec::new())
    }
}

struct App;

impl Host<()> for App {
    fn kept<'a>(&'a self, post: &'a Kept) -> Element<'a, ()> {
        text(post.text.as_str()).into()
    }
}

#[test]
fn loading_reads_once_and_hands_the_posts_over() {
    let mut s = Screen::default();
    let _ = s.update(Msg::Load("did:plc:me".into()), &Fake);
    assert!(s.loading);
    assert!(
        s.update(Msg::Load("did:plc:me".into()), &Fake)
            .events
            .is_empty()
    );
    let out = s.update(Msg::Loaded(Ok(Vec::new())), &Fake);
    assert!(!s.loading && matches!(out.events[..], [Event::Posts(_)]));
    let _ = s.update(Msg::Loaded(Err(ApiError::Other("offline".into()))), &Fake);
    assert!(s.error.is_some());
}

#[test]
fn shows_totals_and_when_you_post() {
    let stats = Stats {
        posts: 4,
        replies: 2,
        likes: 30,
        per_day: vec![1; crate::model::STATS_DAYS],
        per_weekday: [0, 0, 5, 0, 0, 0, 1],
        per_hour: {
            let mut h = [0; 24];
            h[9] = 6;
            h
        },
        ..Stats::default()
    };
    let screen = Screen::default();
    let view = View {
        screen: &screen,
        stats: &stats,
        host: &App,
    };
    let mut ui = simulator(view.view());
    assert!(ui.find("30").is_ok());
    assert!(
        ui.find("You post most on Wednesdays, around 09:00.")
            .is_ok()
    );

    let empty = Stats::default();
    let view = View {
        screen: &screen,
        stats: &empty,
        host: &App,
    };
    let mut ui = simulator(view.view());
    assert!(ui.find("You haven't posted in the last month.").is_ok());
}
