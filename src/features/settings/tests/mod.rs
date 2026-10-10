//! The Settings screen: what each control asks the shell for, and the
//! remove-everything confirmation.

use super::*;
use crate::model::{AlertKind, Alerts};
use iced::widget::text;
use iced::{Element, Size};
use iced_test::Simulator;

/// The parts of Settings the app itself draws, as plain stand-ins.
struct Plain;

impl Host<Msg> for Plain {
    fn translation(&self) -> Element<'_, Msg> {
        text("Translate into").into()
    }

    fn privacy(&self) -> Element<'_, Msg> {
        text("Privacy switches").into()
    }
    fn chats(&self) -> Element<'_, Msg> {
        text("Who can message you").into()
    }

    fn accounts(&self) -> Element<'_, Msg> {
        text("@me.test").into()
    }
}

fn prefs() -> Prefs {
    Prefs {
        appearance: 1,
        appearances: vec!["Dark", "Light", "System"],
        zoom: 1.0,
        zooms: &[0.9, 1.0, 1.5],
        sans_reading: false,
        reduce_motion: true,
        alt_reminder: true,
        data_saver: false,
        deck: false,
        post_lang: "en".into(),
        muted_words: 2,
        media: vec![Service {
            id: "youtube",
            name: "YouTube",
            description: "Shows a card for YouTube links.",
            on: false,
        }],
        services: vec![Service {
            id: "live",
            name: "Live streams",
            description: "Watch live streams.",
            on: false,
        }],
        alerts: Alerts::default(),
        close_to_tray: Some(true),
        version: "0.1.0",
        update: UpdateCheck::Off,
    }
}

fn screen(s: &Screen) -> Element<'_, Msg> {
    screen_with(s, prefs())
}

fn screen_with(s: &Screen, prefs: Prefs) -> Element<'_, Msg> {
    View {
        screen: s,
        prefs,
        host: &Plain,
        lift: |m| m,
    }
    .body()
}

/// Settings is a long screen, so the test window is tall enough to hold all of it.
fn simulator(s: &Screen) -> Simulator<'_, Msg> {
    Simulator::with_size(
        iced::Settings::default(),
        Size::new(1024.0, 4000.0),
        screen(s),
    )
}

fn press(s: &Screen, what: &str) -> Vec<Msg> {
    let mut ui = simulator(s);
    ui.click(what).unwrap();
    ui.into_messages().collect()
}

#[test]
fn every_section_is_there_with_the_cards_the_app_draws() {
    let s = Screen::default();
    let mut ui = simulator(&s);
    for heading in [
        "POSTING",
        "TRANSLATION",
        "APPEARANCE",
        "LAYOUT",
        "ACCESSIBILITY",
        "CHATS",
        "NOTIFICATIONS",
        "MODERATION",
        "EXTERNAL MEDIA",
        "INTEGRATIONS",
        "ACCOUNTS",
        "ABOUT",
        "KEYBOARD SHORTCUTS",
    ] {
        assert!(ui.find(heading).is_ok(), "{heading} is missing");
    }
    assert!(ui.find("Translate into").is_ok(), "the Translation card");
    assert!(ui.find("Who can message you").is_ok(), "the Chats card");
    assert!(ui.find("@me.test").is_ok(), "the account list");
    assert!(ui.find("YouTube").is_ok(), "the media row");
    assert!(ui.find("Live streams").is_ok(), "the integration row");
    assert!(
        ui.find("Content filters, 2 muted words, muted and blocked accounts.")
            .is_ok()
    );
}

#[test]
fn picking_a_colour_scheme_asks_for_it_by_position() {
    let s = Screen::default();
    assert!(matches!(press(&s, "Dark")[..], [Msg::Appearance(0)]));
    assert!(matches!(press(&s, "System")[..], [Msg::Appearance(2)]));
}

#[test]
fn picking_a_size_asks_for_that_zoom() {
    let s = Screen::default();
    assert!(matches!(press(&s, "150%")[..], [Msg::Zoom(z)] if (z - 1.5).abs() < 0.001));
}

#[test]
fn the_moderation_row_opens_moderation() {
    let mut s = Screen::default();
    assert!(matches!(press(&s, "Moderation")[..], [Msg::Moderation]));
    assert_eq!(s.update(Msg::Moderation).events, [Event::Moderation]);
}

#[test]
fn preferences_are_handed_to_the_shell_to_save() {
    let mut s = Screen::default();
    assert_eq!(s.update(Msg::Zoom(1.5)).events, [Event::Zoom(1.5)]);
    assert_eq!(s.update(Msg::Appearance(2)).events, [Event::Appearance(2)]);
    assert_eq!(
        s.update(Msg::SansReading(true)).events,
        [Event::SansReading(true)]
    );
    assert_eq!(
        s.update(Msg::ReduceMotion(false)).events,
        [Event::ReduceMotion(false)]
    );
    assert_eq!(s.update(Msg::Deck(true)).events, [Event::Deck(true)]);
    assert_eq!(
        s.update(Msg::PostLang("de".into())).events,
        [Event::PostLang("de".into())]
    );
    assert_eq!(
        s.update(Msg::Integration("youtube", true)).events,
        [Event::Integration("youtube", true)]
    );
    assert_eq!(
        s.update(Msg::Alert(AlertKind::Likes, true)).events,
        [Event::Alert(AlertKind::Likes, true)]
    );
    assert_eq!(
        s.update(Msg::CloseToTray(false)).events,
        [Event::CloseToTray(false)]
    );
    assert_eq!(
        s.update(Msg::AltReminder(false)).events,
        [Event::AltReminder(false)]
    );
    assert_eq!(
        s.update(Msg::DataSaver(true)).events,
        [Event::DataSaver(true)]
    );
    assert_eq!(
        s.update(Msg::CheckUpdates(true)).events,
        [Event::CheckUpdates(true)]
    );
}

#[test]
fn about_shows_the_version_and_reports_a_problem() {
    let mut s = Screen::default();
    {
        let mut ui = simulator(&s);
        assert!(ui.find("BiFrost 0.1.0").is_ok());
        assert!(ui.find("Checking…").is_err(), "the check is off");
    }
    assert!(matches!(press(&s, "Report…")[..], [Msg::ReportProblem]));
    assert_eq!(s.update(Msg::ReportProblem).events, [Event::ReportProblem]);
}

#[test]
fn the_update_check_says_what_it_found() {
    let mut s = Screen::default();
    let with = |update| Prefs { update, ..prefs() };
    for (update, words) in [
        (UpdateCheck::Checking, "Checking…"),
        (UpdateCheck::UpToDate, "You have the newest version."),
        (
            UpdateCheck::Failed("no connection".into()),
            "Couldn't check for updates: no connection",
        ),
        (
            UpdateCheck::Available {
                version: "0.2.0".into(),
                url: "https://x/0.2.0".into(),
            },
            "BiFrost 0.2.0 is out.",
        ),
    ] {
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            Size::new(1024.0, 4000.0),
            screen_with(&s, with(update)),
        );
        assert!(ui.find(words).is_ok(), "{words} is missing");
    }
    let mut ui = Simulator::with_size(
        iced::Settings::default(),
        Size::new(1024.0, 4000.0),
        screen_with(
            &s,
            with(UpdateCheck::Available {
                version: "0.2.0".into(),
                url: "https://x/0.2.0".into(),
            }),
        ),
    );
    ui.click("Download").unwrap();
    let msgs: Vec<Msg> = ui.into_messages().collect();
    assert!(matches!(&msgs[..], [Msg::OpenRelease(u)] if u == "https://x/0.2.0"));
    assert_eq!(
        s.update(Msg::OpenRelease("https://x/0.2.0".into())).events,
        [Event::Browse("https://x/0.2.0".into())]
    );
}

#[test]
fn each_notification_kind_has_a_switch() {
    let s = Screen::default();
    let mut ui = simulator(&s);
    for name in [
        "Mentions and quotes",
        "Replies",
        "Likes",
        "Reposts",
        "New followers",
        "Direct messages",
    ] {
        assert!(ui.find(name).is_ok(), "{name} is missing");
    }
}

#[test]
fn the_tray_switch_shows_only_where_there_is_a_tray() {
    let s = Screen::default();
    let mut ui = simulator(&s);
    assert!(ui.find("Keep running in the tray").is_ok());
    let no_tray = Prefs {
        close_to_tray: None,
        ..prefs()
    };
    let mut ui = Simulator::with_size(
        iced::Settings::default(),
        Size::new(1024.0, 4000.0),
        screen_with(&s, no_tray),
    );
    assert!(ui.find("Keep running in the tray").is_err());
}

#[test]
fn integrations_are_listed_only_when_there_are_some() {
    let s = Screen::default();
    let none = Prefs {
        services: Vec::new(),
        ..prefs()
    };
    let mut ui = Simulator::with_size(
        iced::Settings::default(),
        Size::new(1024.0, 4000.0),
        screen_with(&s, none),
    );
    assert!(ui.find("INTEGRATIONS").is_err());
    assert!(ui.find("EXTERNAL MEDIA").is_ok());
}

#[test]
fn removing_everything_asks_first() {
    let mut s = Screen::default();
    assert!(!s.confirming());
    assert!(matches!(press(&s, "Remove…")[..], [Msg::RemoveAll]));

    assert!(s.update(Msg::RemoveAll).events.is_empty(), "nothing yet");
    assert!(s.confirming());
    let mut ui = simulator(&s);
    assert!(ui.find("Remove everything?").is_ok());
    assert!(ui.find("Remove…").is_err(), "the card now asks instead");
}

#[test]
fn cancelling_leaves_everything_alone() {
    let mut s = Screen::default();
    let _ = s.update(Msg::RemoveAll);
    assert!(matches!(
        press(&s, "Cancel")[..],
        [Msg::ConfirmRemoveAll(false)]
    ));
    assert!(s.update(Msg::ConfirmRemoveAll(false)).events.is_empty());
    assert!(!s.confirming(), "back to the resting card");
}

#[test]
fn confirming_wipes_once() {
    let mut s = Screen::default();
    let _ = s.update(Msg::RemoveAll);
    assert!(matches!(
        press(&s, "Remove everything")[..],
        [Msg::ConfirmRemoveAll(true)]
    ));
    assert_eq!(s.update(Msg::ConfirmRemoveAll(true)).events, [Event::Wipe]);
    assert!(!s.confirming());
    assert!(
        s.update(Msg::ConfirmRemoveAll(true)).events == [Event::Wipe],
        "the shell decides what to do; the card only asks"
    );
}
