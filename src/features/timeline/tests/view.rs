//! Drawing a timeline: replies under the post they answer, only the posts near the
//! scroll position, and what an empty or failed timeline says.

use super::*;
use crate::components::Window;
use iced::Element;
use iced::widget::text;
use iced_test::simulator;

struct Plain;

impl<'a> Host<'a, Msg> for Plain {
    fn post(&self, post: &'a PostView, shape: Shape<'a>) -> Element<'a, Msg> {
        let mut line = post.record.text.clone();
        if shape.line_above {
            line.insert_str(0, "| ");
        }
        if shape.line_below {
            line.push_str(" |");
        }
        if let Some(a) = shape.reply_parent {
            line = format!("{line} (to @{})", a.handle);
        }
        text(line).into()
    }
    fn hidden(&self, _: &PostView) -> bool {
        false
    }
    fn retry(&self, why: String) -> Element<'a, Msg> {
        text(why).into()
    }
}

fn view<'a>(t: &'a Timelines, key: &Key, offset: f32) -> Element<'a, Msg> {
    View {
        timelines: t,
        key: Some(key.clone()),
        selected: None,
        window: Window {
            offset,
            screen: 400.0,
        },
        post_id: |i| iced::widget::Id::from(format!("post-{i}")),
        host: Plain,
        lift: |m| m,
    }
    .view()
}

#[test]
fn replies_show_under_the_post_they_answer() {
    let mut t = Timelines::default();
    loaded(&mut t, &HOME, vec![reply(2, 1), reply(3, 2), item(4)], None);
    let mut ui = simulator(view(&t, &HOME, 0.0));
    assert!(ui.find("post 1 |").is_ok(), "the parent, joined below");
    assert!(ui.find("| post 2 |").is_ok(), "answered by the next one");
    assert!(ui.find("| post 3").is_ok(), "its parent is just above");
    assert!(ui.find("post 4").is_ok());
    assert!(ui.find("You're all caught up.").is_ok());
}

#[test]
fn only_posts_near_the_scroll_position_are_drawn() {
    let mut t = Timelines::default();
    let items = (1..=30).rev().map(item).collect();
    loaded(&mut t, &Key::Bookmarks, items, Some("c"));
    // Posts are assumed 240 tall, and two screens above 4800 and three below are
    // drawn: the 17th to the 26th post.
    let mut ui = simulator(view(&t, &Key::Bookmarks, 4800.0));
    assert!(ui.find("post 15").is_err());
    assert!(ui.find("post 14").is_ok());
    assert!(ui.find("post 5").is_ok());
    assert!(ui.find("post 4").is_err());
}

#[test]
fn an_empty_or_failed_timeline_says_so() {
    let mut t = Timelines::default();
    assert!(
        simulator(view(&t, &Key::Bookmarks, 0.0))
            .find("Loading…")
            .is_ok()
    );
    loaded(&mut t, &Key::Bookmarks, Vec::new(), None);
    assert!(
        simulator(view(&t, &Key::Bookmarks, 0.0))
            .find("Nothing here yet.")
            .is_ok()
    );
    let _ = run(
        &mut t,
        Msg::Load {
            key: Key::Bookmarks,
            force: true,
        },
    );
    let _ = run(
        &mut t,
        Msg::Loaded(
            Key::Bookmarks,
            Load::First,
            Err(ApiError::Other("down".into())),
        ),
    );
    let mut ui = simulator(view(&t, &Key::Bookmarks, 0.0));
    assert!(
        ui.find(ApiError::Other("down".into()).friendly().as_str())
            .is_ok()
    );
}
