//! When a scheduled post goes out: the quick choices and a typed time.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, NaiveTime, TimeZone, Utc};

/// Times offered in the Schedule menu, from `now`: in an hour, this evening (while
/// it is still ahead), tomorrow morning, and next Monday morning.
pub fn presets(now: DateTime<Local>) -> Vec<(String, DateTime<Utc>)> {
    let at = |days: i64, hour: u32| {
        let day = now.date_naive() + Duration::days(days);
        let time = NaiveTime::from_hms_opt(hour, 0, 0)?;
        Local
            .from_local_datetime(&day.and_time(time))
            .earliest()
            .map(|t| t.with_timezone(&Utc))
    };
    let mut out = vec![(
        "In an hour".to_string(),
        (now + Duration::hours(1)).with_timezone(&Utc),
    )];
    if let Some(t) = at(0, 18).filter(|t| *t > now.with_timezone(&Utc) + Duration::hours(1)) {
        out.push(("This evening at 6 PM".into(), t));
    }
    if let Some(t) = at(1, 9) {
        out.push(("Tomorrow at 9 AM".into(), t));
    }
    let to_monday = 7 - i64::from(now.weekday().num_days_from_monday());
    if to_monday > 1
        && let Some(t) = at(to_monday, 9)
    {
        out.push(("Monday at 9 AM".into(), t));
    }
    out
}

/// A time typed as `YYYY-MM-DD HH:MM`, in this computer's time zone, if it is
/// after `now`.
pub fn typed(text: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let t = NaiveDateTime::parse_from_str(text.trim(), "%Y-%m-%d %H:%M").ok()?;
    let t = Local
        .from_local_datetime(&t)
        .earliest()?
        .with_timezone(&Utc);
    (t > now).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_times_ahead_only() {
        // A Wednesday morning.
        let now = Local.with_ymd_and_hms(2026, 10, 7, 8, 0, 0).unwrap();
        let labels: Vec<String> = presets(now).into_iter().map(|p| p.0).collect();
        assert_eq!(
            labels,
            [
                "In an hour",
                "This evening at 6 PM",
                "Tomorrow at 9 AM",
                "Monday at 9 AM"
            ]
        );
        // Sunday night: no evening left, and tomorrow is already Monday.
        let late = Local.with_ymd_and_hms(2026, 10, 11, 22, 0, 0).unwrap();
        let labels: Vec<String> = presets(late).into_iter().map(|p| p.0).collect();
        assert_eq!(labels, ["In an hour", "Tomorrow at 9 AM"]);
        for (_, t) in presets(late) {
            assert!(t > late.with_timezone(&Utc));
        }
    }

    #[test]
    fn reads_typed_times_in_the_future() {
        let now = Local
            .with_ymd_and_hms(2026, 10, 7, 8, 0, 0)
            .unwrap()
            .with_timezone(&Utc);
        let t = typed(" 2026-10-08 14:30 ", now).unwrap();
        assert_eq!(t.with_timezone(&Local).format("%H:%M").to_string(), "14:30");
        assert_eq!(typed("2026-10-01 14:30", now), None, "in the past");
        assert_eq!(typed("tomorrow", now), None);
    }
}
