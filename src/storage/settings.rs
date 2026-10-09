//! Local app settings, stored as JSON next to the account index.

use super::later::Later;
use crate::model::{DeckColumn, DeckWidth, Did};
use crate::platform::paths;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Integration id to enabled flag. Anything missing is off: integrations are
    /// opt-in.
    #[serde(default)]
    pub integrations: BTreeMap<String, bool>,
    /// Language code new posts are tagged with. None follows the system language.
    #[serde(default)]
    pub post_lang: Option<String>,
    /// Hashtags you have posted with, most recent first, for suggestions.
    #[serde(default)]
    pub recent_tags: Vec<String>,
    /// Searches you have run, most recent first.
    #[serde(default)]
    pub recent_searches: Vec<String>,
    /// How large everything is drawn, 1.0 being normal.
    #[serde(default = "normal_zoom")]
    pub zoom: f32,
    /// Keeps GIFs still until opened.
    #[serde(default)]
    pub reduce_motion: bool,
    /// Videos start without sound.
    #[serde(default)]
    pub video_muted: bool,
    /// Home shows several timelines side by side.
    #[serde(default)]
    pub deck: bool,
    /// Each account's columns, by DID, left to right.
    #[serde(default)]
    pub deck_columns: BTreeMap<Did, Vec<DeckColumn>>,
    #[serde(default)]
    pub deck_width: DeckWidth,
    /// Where post translations come from.
    #[serde(default)]
    pub translate_service: crate::model::TranslateService,
    /// Language posts are translated into. None follows the post language.
    #[serde(default)]
    pub translate_to: Option<String>,
    /// A LibreTranslate server's address.
    #[serde(default)]
    pub translate_server: String,
    #[serde(default)]
    pub appearance: Appearance,
    /// Post text in a proportional font instead of the monospace one.
    #[serde(default)]
    pub sans_reading: bool,
    /// The suggestion to try columns on a wide window was dismissed or taken.
    #[serde(default)]
    pub deck_hint_seen: bool,
    /// What pops up as an OS notification while BiFrost is in the background.
    #[serde(default)]
    pub alerts: crate::model::Alerts,
    /// Closing the window keeps BiFrost running in the system tray, where there is one.
    #[serde(default = "yes")]
    pub close_to_tray: bool,
    /// What each feed leaves out, by `FeedSource::id`.
    #[serde(default)]
    pub feed_filters: BTreeMap<String, crate::model::FeedFilter>,
    /// Posting with an image that has no alt text asks first.
    #[serde(default = "yes")]
    pub alt_reminder: bool,
    /// Uses less data: pictures load only once drawn, GIFs stay still and link
    /// cards come without pictures.
    #[serde(default)]
    pub data_saver: bool,
}

/// Dark or light colours, or whichever the system uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    System,
}

impl Appearance {
    pub const ALL: [Appearance; 3] = [Appearance::Dark, Appearance::Light, Appearance::System];

    pub fn label(self) -> &'static str {
        match self {
            Appearance::Dark => "Dark",
            Appearance::Light => "Light",
            Appearance::System => "Match system",
        }
    }
}

fn normal_zoom() -> f32 {
    1.0
}

fn yes() -> bool {
    true
}

/// The sizes offered for `Settings::zoom`.
pub const ZOOM_STEPS: [f32; 6] = [0.9, 1.0, 1.1, 1.25, 1.5, 1.75];

impl Default for Settings {
    fn default() -> Self {
        Self {
            integrations: BTreeMap::new(),
            post_lang: None,
            recent_tags: Vec::new(),
            recent_searches: Vec::new(),
            zoom: normal_zoom(),
            reduce_motion: false,
            video_muted: false,
            deck: false,
            deck_columns: BTreeMap::new(),
            deck_width: DeckWidth::Normal,
            translate_service: crate::model::TranslateService::default(),
            translate_to: None,
            translate_server: String::new(),
            appearance: Appearance::default(),
            sans_reading: false,
            deck_hint_seen: false,
            alerts: crate::model::Alerts::default(),
            close_to_tray: true,
            feed_filters: BTreeMap::new(),
            alt_reminder: true,
            data_saver: false,
        }
    }
}

impl Settings {
    /// The next size up (`step` 1) or down (-1) from the current zoom.
    pub fn zoom_step(&self, step: i32) -> f32 {
        let i = ZOOM_STEPS
            .iter()
            .position(|z| *z >= self.zoom - 0.001)
            .unwrap_or(ZOOM_STEPS.len() - 1) as i32;
        ZOOM_STEPS[(i + step).clamp(0, ZOOM_STEPS.len() as i32 - 1) as usize]
    }

    fn path() -> Option<PathBuf> {
        Some(paths::config_dir()?.join("settings.json"))
    }

    /// Loads settings, falling back to defaults if the file is missing or unreadable.
    pub fn load() -> Self {
        Self::path().map_or_else(Self::default, |p| Self::read(&p))
    }

    fn read(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Saves settings in the background, so a click never waits on the disk. A save
    /// that finishes late never overwrites a newer one.
    pub fn save(&self) -> std::io::Result<()> {
        static FILE: Later = Later::new("settings");
        let Some(path) = Self::path() else {
            return Ok(());
        };
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        FILE.write(path, json);
        Ok(())
    }

    /// Deletes the settings file, for "Remove all BiFrost data".
    pub fn delete_file() {
        if let Some(path) = Self::path() {
            // Its folder goes too, last, and that failure is the one reported.
            let _ = std::fs::remove_file(path);
        }
    }

    /// Drops what belongs to one account, when it logs out.
    pub fn forget_account(&mut self, did: &str) {
        self.deck_columns.remove(did);
        // Not kept per account, so they may be this account's.
        self.recent_searches.clear();
        self.recent_tags.clear();
    }

    /// Puts `tags` at the front of the recent hashtags.
    pub fn remember_tags(&mut self, tags: impl IntoIterator<Item = String>) {
        for tag in tags {
            self.recent_tags.retain(|t| !t.eq_ignore_ascii_case(&tag));
            self.recent_tags.insert(0, tag);
        }
        self.recent_tags.truncate(50);
    }

    pub fn remember_search(&mut self, q: &str) {
        self.recent_searches.retain(|s| !s.eq_ignore_ascii_case(q));
        self.recent_searches.insert(0, q.to_string());
        self.recent_searches.truncate(10);
    }

    pub fn integration_enabled(&self, id: &str) -> bool {
        self.integrations.get(id).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(s: &Settings) -> serde_json::Value {
        serde_json::to_value(s).unwrap()
    }

    #[test]
    fn settings_read_back_as_saved_and_bad_files_give_defaults() {
        let dir = std::env::temp_dir().join(format!("bifrost-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        let mut s = Settings {
            zoom: 1.25,
            reduce_motion: true,
            close_to_tray: false,
            appearance: Appearance::Light,
            ..Settings::default()
        };
        s.integrations.insert("youtube".into(), true);
        s.remember_search("cats");
        std::fs::write(&path, serde_json::to_vec_pretty(&s).unwrap()).unwrap();
        assert_eq!(json(&Settings::read(&path)), json(&s));

        // A file from an older BiFrost, missing newer settings, keeps what it has.
        std::fs::write(&path, br#"{"zoom": 1.5, "retired": 1}"#).unwrap();
        let old = Settings::read(&path);
        assert!((old.zoom - 1.5).abs() < f32::EPSILON);
        assert!(old.close_to_tray, "new settings take their defaults");

        for bad in [&b"not json"[..], b"", b"[]"] {
            std::fs::write(&path, bad).unwrap();
            assert_eq!(json(&Settings::read(&path)), json(&Settings::default()));
        }
        assert_eq!(
            json(&Settings::read(&dir.join("missing.json"))),
            json(&Settings::default())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn integrations_are_off_until_switched_on() {
        let mut s = Settings::default();
        assert!(!s.integration_enabled("youtube"));
        s.integrations.insert("youtube".into(), true);
        assert!(s.integration_enabled("youtube"));
    }

    #[test]
    fn zoom_steps_up_and_down_and_stops_at_the_ends() {
        let s = Settings::default();
        assert!((s.zoom_step(1) - 1.1).abs() < f32::EPSILON);
        assert!((s.zoom_step(-1) - 0.9).abs() < f32::EPSILON);
        assert!((s.zoom_step(-5) - 0.9).abs() < f32::EPSILON);
        let big = Settings {
            zoom: 1.75,
            ..Settings::default()
        };
        assert!((big.zoom_step(1) - 1.75).abs() < f32::EPSILON);
    }

    #[test]
    fn recent_tags_and_searches_move_to_the_front_once_and_are_capped() {
        let mut s = Settings::default();
        s.remember_tags(["Art".to_string(), "cats".to_string()]);
        s.remember_tags(["art".to_string()]);
        assert_eq!(s.recent_tags, ["art", "cats"]);
        s.remember_tags((0..60).map(|i| format!("t{i}")));
        assert_eq!(s.recent_tags.len(), 50);
        assert_eq!(s.recent_tags[0], "t59");
        for i in 0..12 {
            s.remember_search(&format!("q{i}"));
        }
        s.remember_search("Q5");
        assert_eq!(s.recent_searches.len(), 10);
        assert_eq!(s.recent_searches[0], "Q5");
        assert_eq!(
            s.recent_searches
                .iter()
                .filter(|q| q.eq_ignore_ascii_case("q5"))
                .count(),
            1
        );
    }

    #[test]
    fn signing_out_forgets_recent_searches_and_tags() {
        let mut s = Settings::default();
        s.remember_search("cats");
        s.remember_tags(["art".to_string()]);
        s.forget_account("did:plc:a");
        assert!(s.recent_searches.is_empty());
        assert!(s.recent_tags.is_empty());
    }
}
