//! The preferences the Settings screen draws, as the shell hands them over.

use crate::model::Alerts;

/// One integration, as a row in Settings.
#[derive(Debug, Clone)]
pub struct Service {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub on: bool,
}

/// A snapshot of the preferences shown in Settings. The shell builds it from
/// `crate::storage::settings` and applies the screen's events back onto it.
#[derive(Debug, Clone)]
pub struct Prefs {
    /// The chosen colour scheme, as an index into `appearances`.
    pub appearance: usize,
    /// The colour schemes to choose from, in order.
    pub appearances: Vec<&'static str>,
    pub zoom: f32,
    /// The zoom steps to choose from, in order.
    pub zooms: &'static [f32],
    pub sans_reading: bool,
    pub reduce_motion: bool,
    pub alt_reminder: bool,
    pub data_saver: bool,
    /// Columns on Home.
    pub deck: bool,
    /// The language new posts are tagged with.
    pub post_lang: String,
    /// How many muted words there are, for the Moderation row.
    pub muted_words: usize,
    /// Services whose links show as players or GIFs.
    pub media: Vec<Service>,
    /// Services with screens of their own.
    pub services: Vec<Service>,
    /// Which kinds of activity pop up as OS notifications.
    pub alerts: Alerts,
    /// Whether closing the window keeps BiFrost in the tray, or `None` where this
    /// system has no tray.
    pub close_to_tray: Option<bool>,
    /// This build's version.
    pub version: &'static str,
    /// The opt-in update check and what it found.
    pub update: UpdateCheck,
}

/// The opt-in check for a newer BiFrost, as Settings shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheck {
    Off,
    Checking,
    UpToDate,
    /// A newer version is out; `url` is its release page.
    Available {
        version: String,
        url: String,
    },
    /// The check didn't get an answer, in words for people.
    Failed(String),
}
