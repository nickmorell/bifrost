//! Moderation: muted words, the filters and links on the screen, and writing and
//! sending a report.

use super::*;
use crate::api::service::ModerationApi;
use crate::api::{ApiError, ReportReason, ReportSubject};
use crate::features;
use crate::model::{Did, LabelVisibility, PeopleKind, PrefEdit, Preferences};
use iced_test::simulator;
use serde_json::json;

/// Accepts every report. Reports go out only when their task runs.
#[derive(Clone)]
struct Fake;

impl ModerationApi for Fake {
    async fn report(&self, _: &ReportSubject, _: ReportReason, _: &str) -> Result<(), ApiError> {
        Ok(())
    }
}

fn run(m: &mut Moderation, msg: Msg) -> features::Update<Msg, Event> {
    m.update(msg, &Fake)
}

#[test]
fn muted_words_are_added_unless_blank() {
    let mut m = Moderation::default();
    let _ = run(&mut m, Msg::WordInput(" # ".into()));
    assert!(run(&mut m, Msg::AddWord).events.is_empty());
    let _ = run(&mut m, Msg::WordInput("x".repeat(150)));
    assert_eq!(m.muted_word.chars().count(), 100);
    let _ = run(&mut m, Msg::WordInput("#spoilers".into()));
    assert!(matches!(
        &run(&mut m, Msg::AddWord).events[..],
        [Event::Edit(PrefEdit::MuteWord(w))] if w == "#spoilers"
    ));
    assert!(m.muted_word.is_empty());
}

#[test]
fn a_report_needs_a_reason_and_closes_once_sent() {
    let mut m = Moderation::default();
    let _ = run(
        &mut m,
        Msg::Report(ReportSubject::Account(Did::from("did:plc:x"))),
    );
    assert!(m.reporting());
    assert_eq!(run(&mut m, Msg::Submit).task.units(), 0, "no reason yet");
    let _ = run(&mut m, Msg::Reason(ReportReason::Spam));
    let _ = run(&mut m, Msg::Details("y".repeat(2500)));
    assert_eq!(m.report.as_ref().unwrap().details.len(), 2000);
    assert_eq!(run(&mut m, Msg::Submit).task.units(), 1);
    assert!(m.report.as_ref().unwrap().sending);
    let events = run(&mut m, Msg::Sent(Ok(()))).events;
    assert!(matches!(&events[..], [Event::Toast(t)] if t.starts_with("Report sent")));
    assert!(!m.reporting());
    let _ = run(
        &mut m,
        Msg::Report(ReportSubject::Account(Did::from("did:plc:x"))),
    );
    let events = run(&mut m, Msg::Sent(Err(ApiError::Other("down".into())))).events;
    assert!(matches!(&events[..], [Event::Failed(_)]));
}

#[test]
fn the_report_dialog_picks_a_reason_and_sends() {
    let mut m = Moderation::default();
    assert!(m.dialog().is_none());
    let _ = run(
        &mut m,
        Msg::Report(ReportSubject::Account(Did::from("did:plc:x"))),
    );
    let _ = run(&mut m, Msg::Reason(ReportReason::Rude));
    let mut ui = simulator(m.dialog().unwrap());
    assert!(ui.find("Report this account").is_ok());
    ui.click("Spam").unwrap();
    ui.click("Send report").unwrap();
    ui.click("Cancel").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Reason(ReportReason::Spam), Msg::Submit, Msg::Cancel]
    ));
}

#[test]
fn the_screen_edits_filters_and_words_and_links_to_accounts() {
    let prefs = Preferences::from_json(&[
        json!({"$type": "app.bsky.actor.defs#adultContentPref", "enabled": false}),
        json!({"$type": "app.bsky.actor.defs#mutedWordsPref", "items": [
            {"value": "spoilers", "targets": ["content"]}
        ]}),
    ]);
    let m = Moderation::default();
    let mut ui = simulator(
        Screen {
            moderation: &m,
            prefs: &prefs,
        }
        .view(),
    );
    // Adult filters stay hidden while adult content is off.
    assert!(ui.find("Hidden").is_ok());
    assert!(ui.find("spoilers").is_ok());
    ui.click("Blocked accounts").unwrap();
    let msgs: Vec<Msg> = ui.into_messages().collect();
    assert!(matches!(&msgs[..], [Msg::People(PeopleKind::Blocked)]));
    let mut ui = simulator(
        Screen {
            moderation: &m,
            prefs: &prefs,
        }
        .view(),
    );
    ui.click("Warn").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Edit(PrefEdit::Label(_, LabelVisibility::Warn))]
    ));
}
