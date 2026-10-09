//! Long text sent as a thread.

use super::*;
use crate::features::composer::state::Attached;

fn long() -> String {
    "A sentence that runs on for a while, to fill posts. ".repeat(12)
}

#[test]
fn long_text_offers_a_thread_and_previews_it() {
    let mut c = Composer::default();
    typed(&mut c, &long());
    assert!(!c.can_post(&c.text()), "too long for one post");
    let app = app();
    let mut ui = simulator(dialog(&c, &app));
    ui.click("Split into 3 posts").unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::ToggleThread]
    ));

    events(&mut c, Msg::ToggleThread);
    assert!(c.can_post(&c.text()));
    let mut ui = simulator(dialog(&c, &app));
    assert!(ui.find("A thread of 3 posts").is_ok());
    assert!(ui.find("3 posts").is_ok());
    ui.click("Don't split").unwrap();
}

#[test]
fn the_thread_keeps_reply_and_quote_on_the_first_post_and_images_where_picked() {
    let mut c = Composer::reply(post(
        "at://did:plc:a/app.bsky.feed.post/2",
        Some("at://did:plc:a/app.bsky.feed.post/1"),
    ));
    typed(&mut c, &long());
    events(&mut c, Msg::ToggleThread);
    c.images.push(Attached::new(image(), "First".into()));
    c.images.push(Attached::new(image(), "Second".into()));
    events(&mut c, Msg::ImagePart(1));
    events(&mut c, Msg::ImagePart(1));

    let posts = c.thread_drafts();
    assert_eq!(posts.len(), 3);
    assert!(posts[0].text.ends_with(" 1/3") && posts[2].text.ends_with(" 3/3"));
    let (root, _) = posts[0].reply.as_ref().unwrap();
    assert_eq!(root.uri.as_str(), "at://did:plc:a/app.bsky.feed.post/1");
    assert!(posts[1].reply.is_none(), "set as each one is sent");
    let alts: Vec<Vec<&str>> = posts
        .iter()
        .map(|p| p.images.iter().map(|i| i.alt.as_str()).collect())
        .collect();
    assert_eq!(alts, [vec!["First"], vec![], vec!["Second"]]);

    // Moving past the last post goes back to the first.
    events(&mut c, Msg::ImagePart(1));
    assert_eq!(c.images[1].part, 0);
}

#[test]
fn a_thread_cannot_be_scheduled() {
    let mut c = Composer::default();
    typed(&mut c, &long());
    events(&mut c, Msg::ToggleThread);
    let at = chrono::Utc::now() + chrono::Duration::hours(1);
    assert!(events(&mut c, Msg::Schedule(at)).is_empty());
}

#[test]
fn posting_an_image_without_alt_text_asks_first() {
    let mut c = Composer::default();
    typed(&mut c, "Look");
    c.images.push(Attached::new(image(), String::new()));
    c.images.push(Attached::new(image(), "A dog".into()));
    events(&mut c, Msg::Submit);
    assert!(c.confirm_alt && !c.posting);
    let app = app();
    let mut ui = simulator(dialog(&c, &app));
    assert!(ui.find("An image has no description.").is_ok());
    ui.click("Add descriptions").unwrap();
    ui.click("Post anyway").unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::AddAlt, Msg::PostAnyway]
    ));
    events(&mut c, Msg::AddAlt);
    assert!(!c.confirm_alt && !c.posting);
    events(&mut c, Msg::Submit);
    events(&mut c, Msg::PostAnyway);
    assert!(c.posting);

    // Switched off in Settings, it posts at once.
    let mut c = Composer::default().with_alt_reminder(false);
    typed(&mut c, "Look");
    c.images.push(Attached::new(image(), String::new()));
    events(&mut c, Msg::Submit);
    assert!(c.posting && !c.confirm_alt);
}

#[test]
fn a_break_line_makes_a_thread_without_asking() {
    let mut c = Composer::default();
    typed(&mut c, "test\n---\nhello");
    assert!(c.threaded());
    assert_eq!(c.thread_drafts().len(), 2);
    let app = app();
    {
        let mut ui = simulator(dialog(&c, &app));
        assert!(ui.find("A thread of 2 posts").is_ok());
    }

    events(&mut c, Msg::ToggleThread);
    assert!(!c.threaded(), "Don't split sends it as it is");
    events(&mut c, Msg::ToggleThread);
    assert!(c.threaded());
}
