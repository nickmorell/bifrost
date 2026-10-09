//! The Privacy switches against a fake Bluesky.

use super::*;
use iced_test::simulator;

#[derive(Clone)]
struct Fake;

impl PrivacyApi for Fake {
    async fn privacy(&self) -> Result<Wanted, ApiError> {
        Ok(Wanted::default())
    }
    async fn set_hide_from_discover(&self, _: bool) -> Result<(), ApiError> {
        Ok(())
    }
    async fn set_hide_logged_out(&self, _: bool) -> Result<(), ApiError> {
        Ok(())
    }
}

fn loaded() -> Privacy {
    let mut p = Privacy::default();
    let _ = p.update(Msg::Loaded(Ok(Wanted::default())), &Fake);
    p
}

#[test]
fn switches_change_at_once_and_a_refusal_reads_them_again() {
    let mut p = Privacy::default();
    let _ = p.update(Msg::Discover(true), &Fake);
    assert!(p.now.is_none(), "nothing to change before it is read");

    let mut p = loaded();
    let _ = p.update(Msg::Discover(true), &Fake);
    let _ = p.update(Msg::LoggedOut(true), &Fake);
    assert_eq!(
        p.now,
        Some(Ok(Wanted {
            hide_from_discover: true,
            hide_logged_out: true
        }))
    );
    let out = p.update(Msg::Saved(Err(ApiError::Other("no".into()))), &Fake);
    assert!(matches!(out.events[..], [Event::Failed(_)]));
}

#[test]
fn the_card_says_posts_stay_public() {
    let p = loaded();
    let mut ui = simulator(p.view());
    assert!(ui.find("Keep my posts out of Discover").is_ok());
    assert!(
        ui.find("Your posts stay public either way: Bluesky has no private accounts, and anyone signed in can see them.")
            .is_ok()
    );
    let mut failed = Privacy::default();
    let _ = failed.update(Msg::Loaded(Err(ApiError::Other("offline".into()))), &Fake);
    let mut ui = simulator(failed.view());
    ui.click("Try again").unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Load]
    ));
}
