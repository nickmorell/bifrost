//! Times as BiFrost shows them.

/// "5m", "3h", "2d", or a date for anything older than a week.
pub fn relative_time(iso: &str) -> String {
    let Ok(t) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return String::new();
    };
    let secs = (chrono::Utc::now() - t.with_timezone(&chrono::Utc))
        .num_seconds()
        .max(0);
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m", secs / 60),
        3600..86_400 => format!("{}h", secs / 3600),
        86_400..604_800 => format!("{}d", secs / 86_400),
        _ => t.format("%b %-d, %Y").to_string(),
    }
}

/// The exact local date and time, for hover text: "Oct 4, 2026 at 3:42 PM".
pub fn full_time(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%b %-d, %Y at %-I:%M %p")
                .to_string()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_time_is_empty() {
        assert_eq!(relative_time("nope"), "");
    }
}
