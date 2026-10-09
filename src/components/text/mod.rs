//! Text in BiFrost's quieter styles.

mod time;
pub use time::{full_time, relative_time};

use crate::theme::{self, size};
use iced::widget::{Text, text};

/// Secondary information: handles, counts, hints.
pub fn muted<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).size(size::SMALL).style(theme::muted)
}

/// The quietest text: timestamps, separators.
pub fn faint<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).size(size::SMALL).style(theme::faint)
}

/// A small uppercase label above a group of settings.
pub fn eyebrow<'a>(content: &str) -> Text<'a> {
    text(content.to_uppercase())
        .size(size::CAPTION)
        .font(theme::MEDIUM)
        .style(theme::faint)
}

/// A count as Bluesky shows it: 1234 -> "1.2K", 12_345 -> "12K", 999_999 -> "999K",
/// 1_250_000 -> "1.2M". Rounds down so a count never shows more than it is.
pub fn compact(n: u64) -> String {
    let (unit, suffix) = match n {
        0..1000 => return n.to_string(),
        1000..1_000_000 => (1000, "K"),
        1_000_000..1_000_000_000 => (1_000_000, "M"),
        _ => (1_000_000_000, "B"),
    };
    let whole = n / unit;
    let tenth = n % unit * 10 / unit;
    if whole >= 10 || tenth == 0 {
        format!("{whole}{suffix}")
    } else {
        format!("{whole}.{tenth}{suffix}")
    }
}

/// The first `max` characters of `s`, with "…" when there were more.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

/// The first `lines` lines of `s`, with "…" on a line of its own when there were more.
pub fn truncate_lines(s: &str, lines: usize) -> String {
    let mut out: Vec<&str> = s.lines().take(lines).collect();
    if s.lines().count() > lines {
        out.push("…");
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Element;
    use iced_test::simulator;

    #[test]
    fn eyebrows_are_upper_case() {
        let view: Element<'_, ()> = eyebrow("Layout").into();
        let mut ui = simulator(view);
        assert!(ui.find("LAYOUT").is_ok());
        assert!(ui.find("Layout").is_err());
    }

    #[test]
    fn compact_counts() {
        assert_eq!(compact(999), "999");
        assert_eq!(compact(1000), "1K");
        assert_eq!(compact(1250), "1.2K");
        assert_eq!(compact(12_345), "12K");
        assert_eq!(compact(999_999), "999K");
        assert_eq!(compact(1_000_000), "1M");
        assert_eq!(compact(1_960_000), "1.9M");
        assert_eq!(compact(2_500_000_000), "2.5B");
    }

    #[test]
    fn long_text_is_cut_to_whole_lines() {
        assert_eq!(truncate("héllo", 5), "héllo");
        assert_eq!(truncate("héllo", 2), "hé…");
        assert_eq!(truncate_lines("a\nb", 3), "a\nb");
        assert_eq!(truncate_lines("a\nb\nc\nd", 2), "a\nb\n…");
    }
}
