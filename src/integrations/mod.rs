//! Optional integrations with outside services.
//!
//! Every integration is off until the user turns it on in Settings, and a disabled
//! integration is never consulted, so it makes no network calls. Most are Bluesky's
//! "external media" sources: they recognise links to a service and show them as a
//! richer card (or, for GIFs, the animation itself) in place of the plain link card.

mod apple_music;
pub mod bandcamp;
mod flickr;
mod freemix;
mod giphy;
mod soundcloud;
mod spotify;
pub mod streamplace;
mod tenor;
mod twitch;
mod vimeo;
mod youtube;

use crate::storage::settings::Settings;
use url::Url;

/// Where an integration is listed in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// A source of links shown as players or cards, as in Bluesky's "External media".
    Media,
    /// A service with screens of its own.
    Service,
}

/// How a card shows its picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Wide across the top, for videos and photos.
    Banner,
    /// A small square beside the text, for album art.
    Cover,
}

/// A richer card an integration renders in place of a plain link preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationCard {
    pub provider: &'static str,
    /// A picture from the service itself. Without one the card uses the link's own
    /// preview picture.
    pub thumbnail_url: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub action_label: String,
    pub open_url: String,
    pub shape: Shape,
}

/// What an integration shows for a link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    Card(IntegrationCard),
    /// The address of an animated GIF to play in place.
    Gif(String),
}

pub trait Integration: Send + Sync {
    /// Stable id used in settings. Never change it once shipped.
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    fn group(&self) -> Group {
        Group::Media
    }

    /// What to show for `url`, if this integration understands it. `title` is the
    /// link's title from the post's own link card, when there is one.
    fn embed(&self, _url: &Url, _title: Option<&str>) -> Option<Shown> {
        None
    }
}

/// All integrations BiFrost knows about, in display order: the media sources in the
/// order Bluesky lists them.
pub fn all() -> Vec<Box<dyn Integration>> {
    vec![
        Box::new(youtube::YouTube),
        Box::new(youtube::Shorts),
        Box::new(vimeo::Vimeo),
        Box::new(twitch::Twitch),
        Box::new(giphy::Giphy),
        Box::new(tenor::Tenor),
        Box::new(tenor::Klipy),
        Box::new(spotify::Spotify),
        Box::new(apple_music::AppleMusic),
        Box::new(soundcloud::SoundCloud),
        Box::new(flickr::Flickr),
        Box::new(bandcamp::Bandcamp),
        Box::new(freemix::FreeMix),
        Box::new(streamplace::Streamplace),
    ]
}

/// Asks each enabled integration, in order, what to show for `url`.
pub fn embed(
    registry: &[Box<dyn Integration>],
    settings: &Settings,
    url: &str,
    title: Option<&str>,
) -> Option<Shown> {
    let parsed = Url::parse(url).ok()?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return None;
    }
    registry
        .iter()
        .filter(|i| settings.integration_enabled(i.id()))
        .find_map(|i| i.embed(&parsed, title))
}

/// The link's host without a leading `www.` or `m.`, lower case.
fn host(url: &Url) -> Option<String> {
    let h = url.host_str()?.to_ascii_lowercase();
    let h = h
        .strip_prefix("www.")
        .or_else(|| h.strip_prefix("m."))
        .unwrap_or(&h);
    Some(h.to_string())
}

/// The link's path, split on `/`, without empty parts.
fn parts(url: &Url) -> Vec<&str> {
    url.path_segments()
        .map(|s| s.filter(|p| !p.is_empty()).collect())
        .unwrap_or_default()
}

/// A card that opens `url` on the service: "Album" on "Spotify", "Listen on Spotify".
fn card(provider: &'static str, what: &str, action: &str, shape: Shape) -> Builder {
    Builder(IntegrationCard {
        provider,
        thumbnail_url: None,
        title: None,
        subtitle: Some(what.to_string()),
        action_label: format!("{action} on {provider}"),
        open_url: String::new(),
        shape,
    })
}

struct Builder(IntegrationCard);

impl Builder {
    fn thumbnail(mut self, url: String) -> Self {
        self.0.thumbnail_url = Some(url);
        self
    }

    fn open(mut self, url: &Url, title: Option<&str>) -> Shown {
        self.0.open_url = url.to_string();
        self.0.title = title.filter(|t| !t.trim().is_empty()).map(str::to_string);
        Shown::Card(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(ids: &[&str]) -> Settings {
        let mut s = Settings::default();
        for id in ids {
            s.integrations.insert((*id).to_string(), true);
        }
        s
    }

    #[test]
    fn disabled_by_default() {
        let registry = all();
        let settings = Settings::default();
        assert!(embed(&registry, &settings, "https://youtu.be/dQw4w9WgXcQ", None).is_none());
        assert!(
            embed(
                &registry,
                &on(&["youtube"]),
                "https://youtu.be/dQw4w9WgXcQ",
                None
            )
            .is_some()
        );
    }

    #[test]
    fn only_the_switched_on_source_answers() {
        let registry = all();
        let spotify = "https://open.spotify.com/album/4aawyAB9vmqN3uQ7FjRGTy";
        assert!(embed(&registry, &on(&["youtube"]), spotify, None).is_none());
        assert!(embed(&registry, &on(&["spotify"]), spotify, None).is_some());
    }

    #[test]
    fn ids_are_unique_and_streamplace_is_the_one_service() {
        let registry = all();
        let mut ids: Vec<_> = registry.iter().map(|i| i.id()).collect();
        assert_eq!(ids.len(), 14);
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 14);
        let services: Vec<_> = registry
            .iter()
            .filter(|i| i.group() == Group::Service)
            .map(|i| i.id())
            .collect();
        assert_eq!(services, [streamplace::ID]);
    }

    #[test]
    fn cards_keep_the_link_and_its_title() {
        let registry = all();
        let url = "https://vimeo.com/76979871";
        let Some(Shown::Card(c)) = embed(&registry, &on(&["vimeo"]), url, Some("A film")) else {
            panic!("no card");
        };
        assert_eq!(c.open_url, url);
        assert_eq!(c.title.as_deref(), Some("A film"));
        assert_eq!(c.action_label, "Watch on Vimeo");
        let Some(Shown::Card(c)) = embed(&registry, &on(&["vimeo"]), url, Some(" ")) else {
            panic!("no card");
        };
        assert_eq!(c.title, None);
    }

    #[test]
    fn hosts_drop_www_and_m() {
        let h = |u: &str| host(&Url::parse(u).unwrap());
        assert_eq!(h("https://www.Twitch.tv/x").as_deref(), Some("twitch.tv"));
        assert_eq!(h("https://m.twitch.tv/x").as_deref(), Some("twitch.tv"));
        assert_eq!(
            h("https://music.apple.com/x").as_deref(),
            Some("music.apple.com")
        );
    }

    #[test]
    fn links_that_are_not_web_pages_are_ignored() {
        let registry = all();
        assert!(embed(&registry, &on(&["vimeo"]), "ftp://vimeo.com/76979871", None).is_none());
    }
}
