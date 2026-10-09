//! Small view helpers shared by the screens.

use super::{App, LinkTarget, Message, account, nav};
use crate::components::{self as ui, Picture};

pub use crate::components::{compact, full_time, relative_time};
use crate::features::Pictures;
use iced::Element;

/// Finds links, @mentions and #tags in plain text such as a profile bio, which has no
/// facets. Returns byte ranges with their targets, in order.
pub fn linkify(text: &str) -> Vec<(std::ops::Range<usize>, LinkTarget)> {
    let mut out = Vec::new();
    let mut start = None;
    // Walk whitespace-separated words with their byte offsets.
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                if let Some(found) = link_in_word(&text[s..i], s) {
                    out.push(found);
                }
                start = None;
            }
            _ => {}
        }
    }
    out
}

fn link_in_word(word: &str, offset: usize) -> Option<(std::ops::Range<usize>, LinkTarget)> {
    let trimmed = word.trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '"', '\'']);
    let end = offset + trimmed.len();
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return (trimmed.len() > 8).then(|| (offset..end, LinkTarget::Url(trimmed.to_string())));
    }
    if let Some(handle) = trimmed.strip_prefix('@') {
        let ok = handle.contains('.')
            && !handle.ends_with('.')
            && handle
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        return ok.then(|| (offset..end, LinkTarget::Profile(handle.to_string())));
    }
    if let Some(ticker) = crate::api::richtext::cashtag(trimmed) {
        return Some((offset..end, LinkTarget::Tag(ticker)));
    }
    if let Some(tag) = trimmed.strip_prefix('#') {
        let ok = !tag.is_empty()
            && !tag.chars().all(|c| c.is_ascii_digit())
            && tag.chars().all(|c| c.is_alphanumeric() || c == '_');
        return ok.then(|| (offset..end, LinkTarget::Tag(tag.to_string())));
    }
    None
}

/// How far the remote image at `url` has got, for the picture components.
pub fn picture_of(app: &App, url: &str) -> Picture {
    app.picture_at(url)
}

impl Pictures for App {
    fn picture(&self, url: Option<&str>) -> Option<Picture> {
        usable(self, url).map(|u| picture_of(self, u))
    }
}

/// The picture's address, unless there is none or it failed to load.
fn usable<'u>(app: &App, url: Option<&'u str>) -> Option<&'u str> {
    url.filter(|u| !app.image_failed(u))
}

/// A round profile picture, or a person silhouette for accounts without one.
pub fn avatar<'a>(app: &'a App, url: Option<&'a str>, size: f32) -> Element<'a, Message> {
    ui::avatar(app.picture(url), size)
}

/// Something that failed to load, with "Sign in again" or "Try again".
pub fn retry<'a>(why: impl Into<String>) -> Element<'a, Message> {
    ui::retry(
        why,
        Message::Account(account::Msg::Changed(Ok(()))),
        Message::Nav(nav::Msg::Retry),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bio_links() {
        let bio = "Rust dev. https://nick.dev, @friend.bsky.social and #rustlang! $tsla #1 @nope";
        let found: Vec<_> = linkify(bio)
            .into_iter()
            .map(|(r, t)| (&bio[r], t))
            .collect();
        assert_eq!(
            found,
            vec![
                (
                    "https://nick.dev",
                    LinkTarget::Url("https://nick.dev".into())
                ),
                (
                    "@friend.bsky.social",
                    LinkTarget::Profile("friend.bsky.social".into())
                ),
                ("#rustlang", LinkTarget::Tag("rustlang".into())),
                ("$tsla", LinkTarget::Tag("$TSLA".into())),
            ]
        );
    }
}
