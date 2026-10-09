//! Facet detection for outgoing posts.
//!
//! Bluesky facets index into the post text by UTF-8 byte offset, which is exactly how
//! Rust strings are indexed, so offsets come straight from `char_indices`.

use unicode_segmentation::UnicodeSegmentation;

/// Bluesky's post length limit, counted in grapheme clusters.
pub const MAX_GRAPHEMES: usize = 300;

pub fn grapheme_len(text: &str) -> usize {
    text.graphemes(true).count()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedKind {
    /// Handle without the leading `@`; must be resolved to a DID before posting.
    Mention(String),
    Link(String),
    /// Tag without the leading `#`.
    Tag(String),
    /// A stock ticker such as `$AAPL`, kept with its `$` and in capitals. Posted as a
    /// tag facet, which is how the `$` tells it apart from a hashtag.
    Cashtag(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub start: usize,
    pub end: usize,
    pub kind: DetectedKind,
}

/// `$` and a ticker: one to five letters, with an optional class such as `.B`.
/// Returns it in capitals, `$` included. Amounts such as `$5` are not tickers.
pub fn cashtag(word: &str) -> Option<String> {
    let ticker = word.strip_prefix('$')?;
    let (base, class) = ticker.split_once('.').unwrap_or((ticker, ""));
    let letters = |s: &str, max: usize| {
        (1..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphabetic())
    };
    (letters(base, 5) && (class.is_empty() || letters(class, 2)))
        .then(|| format!("${}", ticker.to_ascii_uppercase()))
}

/// Finds mentions, links, hashtags and cashtags in `text`.
pub fn detect(text: &str) -> Vec<Detected> {
    let mut out = Vec::new();
    for (start, word) in words(text) {
        let trimmed = word.trim_end_matches(|c: char| ".,;:!?)\"'".contains(c));
        let end = start + trimmed.len();
        if let Some(handle) = trimmed.strip_prefix('@') {
            if is_handle(handle) {
                out.push(Detected {
                    start,
                    end,
                    kind: DetectedKind::Mention(handle.to_lowercase()),
                });
            }
        } else if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
            if trimmed.len() > "https://".len() {
                out.push(Detected {
                    start,
                    end,
                    kind: DetectedKind::Link(trimmed.to_string()),
                });
            }
        } else if let Some(ticker) = cashtag(trimmed) {
            out.push(Detected {
                start,
                end,
                kind: DetectedKind::Cashtag(ticker),
            });
        } else if let Some(tag) = trimmed.strip_prefix('#') {
            // Same rule as the official app: not purely numeric, at most 64 chars.
            if !tag.is_empty()
                && tag.chars().count() <= 64
                && !tag.chars().all(|c| c.is_ascii_digit())
                && !tag.starts_with('#')
            {
                out.push(Detected {
                    start,
                    end,
                    kind: DetectedKind::Tag(tag.to_string()),
                });
            }
        }
    }
    out
}

/// The first link in the text, used for the link card preview.
pub fn first_link(text: &str) -> Option<String> {
    detect(text).into_iter().find_map(|d| match d.kind {
        DetectedKind::Link(url) => Some(url),
        _ => None,
    })
}

/// The `@partial` word ending at the end of `text`, for mention autocomplete.
pub fn trailing_mention(text: &str) -> Option<&str> {
    let last = text.rsplit(char::is_whitespace).next()?;
    let partial = last.strip_prefix('@')?;
    (!partial.is_empty()
        && partial
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-'))
    .then_some(partial)
}

fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut start = None;
    let mut out = Vec::new();
    for (i, c) in text.char_indices() {
        match (c.is_whitespace(), start) {
            (true, Some(s)) => {
                out.push((s, &text[s..i]));
                start = None;
            }
            (false, None) => start = Some(i),
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, &text[s..]));
    }
    out.into_iter()
}

fn is_handle(h: &str) -> bool {
    let labels: Vec<&str> = h.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|l| {
            !l.is_empty()
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && labels
            .last()
            .is_some_and(|tld| tld.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_all_kinds_with_byte_offsets() {
        let text = "héllo @alice.bsky.social, see https://example.com/x. #rust #123";
        let found = detect(text);
        assert_eq!(found.len(), 3);
        assert_eq!(&text[found[0].start..found[0].end], "@alice.bsky.social");
        assert_eq!(
            found[0].kind,
            DetectedKind::Mention("alice.bsky.social".into())
        );
        assert_eq!(&text[found[1].start..found[1].end], "https://example.com/x");
        assert_eq!(found[2].kind, DetectedKind::Tag("rust".into()));
    }

    #[test]
    fn cashtags_are_tickers_not_amounts() {
        let text = "buying $aapl and $BRK.B for $5, not $TOOLONG";
        let found: Vec<DetectedKind> = detect(text).into_iter().map(|d| d.kind).collect();
        assert_eq!(
            found,
            [
                DetectedKind::Cashtag("$AAPL".into()),
                DetectedKind::Cashtag("$BRK.B".into())
            ]
        );
        assert_eq!(cashtag("$nvda").as_deref(), Some("$NVDA"));
        assert_eq!(cashtag("$"), None);
        assert_eq!(cashtag("$1A"), None);
    }

    #[test]
    fn ignores_non_handles() {
        assert!(detect("email me @ home or @nodot").is_empty());
    }

    #[test]
    fn trailing_mention_partial() {
        assert_eq!(trailing_mention("hey @ali"), Some("ali"));
        assert_eq!(trailing_mention("hey @ali "), None);
        assert_eq!(trailing_mention("hey"), None);
    }

    #[test]
    fn counts_graphemes() {
        assert_eq!(grapheme_len("👍🏽a"), 2);
    }
}
