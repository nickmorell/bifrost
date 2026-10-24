//! What views ask the app: the window's layout, the look, the title, and helpers
//! that read across features.

use super::{App, Route, WINDOW, util};
use crate::auth::AccountSummary;
use crate::model::PostView;
use crate::theme::Look;
use crate::{api, integrations, theme};
use iced::Theme;
use std::collections::HashMap;

impl App {
    pub(super) fn title(&self) -> String {
        let name = match &self.me {
            Some(me) => format!("BiFrost · @{}", me.handle),
            None => "BiFrost".into(),
        };
        match self.notifications.unread() + self.chats.unread {
            0 => name,
            n => format!("({}) {name}", util::compact(n)),
        }
    }

    pub(super) fn zoom(&self) -> f32 {
        self.settings.zoom.clamp(0.5, 3.0)
    }

    /// The window's size in layout units, which the zoom setting scales.
    pub(crate) fn layout_size(&self) -> iced::Size {
        self.win.size.map_or(WINDOW, |w| w * (1.0 / self.zoom()))
    }

    pub(crate) fn layout_width(&self) -> f32 {
        self.layout_size().width
    }

    /// The sidebar shrinks to icons in a narrow window.
    pub(crate) fn rail(&self) -> bool {
        self.layout_width() < 900.0
    }

    /// The check to show for an account verified as `v`, as components take it
    /// (`true` for a trusted verifier), unless you turned the checks off.
    pub(crate) fn badge(&self, v: Option<crate::model::Verified>) -> Option<bool> {
        crate::features::badge(v, !self.prefs.hide_verification_badges)
    }

    /// How wide a post in a timeline is drawn: a deck column, or the reading width.
    pub(crate) fn post_width(&self) -> f32 {
        if self.deck_active() {
            self.settings.deck_width.pixels()
        } else {
            theme::layout::READING
        }
    }

    /// The theme and reading font the settings ask for, passed to views.
    pub(crate) fn look(&self) -> Look {
        use crate::storage::settings::Appearance;
        Look {
            palette: match self.settings.appearance {
                Appearance::Dark => &theme::DARK,
                Appearance::Light => &theme::LIGHT,
                Appearance::System => theme::system::palette(self.win.system_mode),
            },
            sans_reading: self.settings.sans_reading,
        }
    }

    pub(super) fn theme(&self) -> Theme {
        self.look().theme()
    }

    pub(crate) fn route(&self) -> &Route {
        self.routes.last().unwrap_or(&Route::Home)
    }

    /// The language new posts are tagged with: the one chosen in Settings, else the
    /// system's when BiFrost offers it, else English.
    pub(super) fn post_lang(&self) -> String {
        self.settings.post_lang.clone().unwrap_or_else(|| {
            crate::platform::locale::language()
                .filter(|l| crate::model::language(l).is_some())
                .unwrap_or_else(|| "en".into())
        })
    }

    pub(crate) fn accounts_list(&self) -> &[AccountSummary] {
        &self.account_list
    }

    /// What a switched-on integration shows for a link, if any recognises it.
    pub(crate) fn integration_embed(
        &self,
        url: &str,
        title: Option<&str>,
    ) -> Option<integrations::Shown> {
        integrations::embed(&self.integrations, &self.settings, url, title)
    }

    /// Hashtags starting with `prefix`: ones you have used first, then the most
    /// common in the posts loaded now.
    pub(crate) fn tag_suggestions(&self, prefix: &str) -> Vec<String> {
        let prefix = prefix.to_lowercase();
        let matches = |t: &str| t.to_lowercase().starts_with(&prefix);
        let mut out: Vec<String> = self
            .settings
            .recent_tags
            .iter()
            .filter(|t| matches(t))
            .cloned()
            .collect();
        let mut counts: HashMap<String, (usize, String)> = HashMap::new();
        for post in self.timelines.posts() {
            for d in api::richtext::detect(&post.record.text) {
                if let api::richtext::DetectedKind::Tag(t) = d.kind
                    && matches(&t)
                {
                    counts.entry(t.to_lowercase()).or_insert((0, t)).0 += 1;
                }
            }
        }
        let mut common: Vec<(usize, String)> = counts.into_values().collect();
        common.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        for (_, t) in common {
            if !out.iter().any(|o| o.eq_ignore_ascii_case(&t)) {
                out.push(t);
            }
        }
        out.retain(|t| !t.eq_ignore_ascii_case(&prefix));
        out.truncate(6);
        out
    }

    pub(crate) fn moderate(
        &self,
        post: &PostView,
        ctx: crate::moderation::Context,
    ) -> crate::moderation::Decision {
        crate::moderation::decide(
            &self.prefs,
            &post.author,
            &post.record,
            &post.labels,
            &self.my_did,
            ctx,
        )
    }
}
