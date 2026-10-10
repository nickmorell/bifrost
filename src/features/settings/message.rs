//! What the Settings screen sends and asks for.

use crate::model::AlertKind;

/// A message from the Settings screen.
#[derive(Debug, Clone)]
pub enum Msg {
    /// A zoom step was picked.
    Zoom(f32),
    /// A colour scheme was picked, as an index into [`super::Prefs::appearances`].
    Appearance(usize),
    /// "Proportional post text" was switched.
    SansReading(bool),
    /// "Reduce motion" was switched.
    ReduceMotion(bool),
    /// Columns on Home were switched.
    Deck(bool),
    /// A language was picked for new posts.
    PostLang(String),
    /// An integration was switched, by its id.
    Integration(&'static str, bool),
    /// An OS notification kind was switched.
    Alert(AlertKind, bool),
    /// "Show message text" was switched.
    MessageText(bool),
    /// "Keep running in the tray" was switched.
    CloseToTray(bool),
    /// "Remind me about alt text" was switched.
    AltReminder(bool),
    /// "Data saver" was switched.
    DataSaver(bool),
    /// The Moderation row was pressed.
    Moderation,
    /// "Check for updates" was switched.
    CheckUpdates(bool),
    /// "Report a problem" was pressed.
    ReportProblem,
    /// The download link of a newer version was pressed, with its release page.
    OpenRelease(String),
    /// "Remove…" was pressed, which asks first.
    RemoveAll,
    /// The confirmation was answered: `true` removes everything.
    ConfirmRemoveAll(bool),
}

/// What the shell has to do for the Settings screen.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Save this zoom level.
    Zoom(f32),
    /// Save this colour scheme, by its index in [`super::Prefs::appearances`].
    Appearance(usize),
    /// Save the proportional-post-text preference.
    SansReading(bool),
    /// Save the reduce-motion preference.
    ReduceMotion(bool),
    /// Turn columns on Home on or off.
    Deck(bool),
    /// Save the language new posts are tagged with.
    PostLang(String),
    /// Turn an integration on or off.
    Integration(&'static str, bool),
    /// Save whether this kind pops up as an OS notification.
    Alert(AlertKind, bool),
    /// Save whether direct message notices show the message.
    MessageText(bool),
    /// Save whether closing the window keeps BiFrost in the tray.
    CloseToTray(bool),
    AltReminder(bool),
    DataSaver(bool),
    /// Open the Moderation screen.
    Moderation,
    /// Save whether BiFrost looks for a newer version, and look now when it does.
    CheckUpdates(bool),
    /// Open a new GitHub issue in the browser, with this build's details filled in.
    ReportProblem,
    /// Open this page in the browser.
    Browse(String),
    /// Sign out everywhere and delete what BiFrost keeps on this computer.
    Wipe,
}
