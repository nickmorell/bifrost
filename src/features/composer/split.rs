//! Splitting long text into a thread of numbered posts.

use crate::api::richtext::{MAX_GRAPHEMES, grapheme_len};
use unicode_segmentation::UnicodeSegmentation;

/// A line with only this on it starts the next post.
pub const BREAK: &str = "---";

/// `text` as posts of at most [`MAX_GRAPHEMES`], each ending in its number (` 1/3`)
/// when there is more than one. Lines of [`BREAK`] split where they are; the rest is
/// split at the last paragraph, sentence or word that fits.
pub fn split(text: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in text.trim_end().split('\n') {
        if line.trim() == BREAK {
            chunks.push(std::mem::take(&mut current));
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }
    chunks.push(current);
    let chunks: Vec<String> = chunks
        .into_iter()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();

    // Numbers take room, and how much depends on how many posts there are.
    let mut count = chunks.len();
    loop {
        let room = MAX_GRAPHEMES - suffix(count, count).len();
        let parts: Vec<String> = chunks.iter().flat_map(|c| fit(c, room)).collect();
        if parts.len() <= 1 {
            return parts;
        }
        if suffix(parts.len(), parts.len()).len() <= suffix(count, count).len() {
            let n = parts.len();
            return parts
                .into_iter()
                .enumerate()
                .map(|(i, p)| format!("{p}{}", suffix(i + 1, n)))
                .collect();
        }
        count = parts.len();
    }
}

fn suffix(i: usize, n: usize) -> String {
    format!(" {i}/{n}")
}

/// `text` cut into pieces of at most `room` graphemes.
fn fit(text: &str, room: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while grapheme_len(rest) > room {
        // Byte offset just past the `room`th grapheme.
        let end = rest
            .grapheme_indices(true)
            .nth(room)
            .map_or(rest.len(), |(i, _)| i);
        let head = &rest[..end];
        let cut = ["\n\n", ". ", "! ", "? ", "\n", " "]
            .iter()
            .find_map(|sep| {
                head.rfind(sep)
                    .map(|i| i + sep.len())
                    .filter(|&i| i > end / 3)
            })
            .unwrap_or(end);
        out.push(rest[..cut].trim().to_string());
        rest = rest[cut..].trim_start();
    }
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_stays_one_post_without_a_number() {
        assert_eq!(split("Hello there.\n"), ["Hello there."]);
        assert!(split("  ").is_empty());
    }

    #[test]
    fn long_text_splits_at_sentences_and_is_numbered() {
        let sentence = "This sentence is exactly fifty graphemes long, ok. ";
        let text = sentence.repeat(10);
        let parts = split(&text);
        assert_eq!(parts.len(), 2);
        for (i, p) in parts.iter().enumerate() {
            assert!(grapheme_len(p) <= MAX_GRAPHEMES, "{p}");
            assert!(p.ends_with(&format!("ok. {}/2", i + 1)), "{p}");
        }
    }

    #[test]
    fn a_break_line_starts_the_next_post() {
        assert_eq!(split("One\n---\nTwo\n --- \n\n"), ["One 1/2", "Two 2/2"]);
    }

    #[test]
    fn a_word_too_long_for_a_post_is_cut_anyway() {
        let word = "é".repeat(700);
        let parts = split(&word);
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| grapheme_len(p) <= MAX_GRAPHEMES));
        assert!(parts[2].ends_with(" 3/3"));
    }
}
