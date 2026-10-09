//! Your own numbers, worked out from the posts of yours kept on this computer.

use super::Kept;
use chrono::{DateTime, Datelike, Duration, FixedOffset, Timelike, Utc};

/// Days the stats look back over.
pub const STATS_DAYS: usize = 30;

/// What your posts of the last [`STATS_DAYS`] days did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    /// Posts that start something, and replies.
    pub posts: usize,
    pub replies: usize,
    /// Likes, reposts and replies they got, as last loaded.
    pub likes: u64,
    pub reposts: u64,
    pub replies_got: u64,
    /// How many you wrote each day, oldest first, ending today.
    pub per_day: Vec<usize>,
    /// Your posts by weekday (Monday first) and hour, in your time zone.
    pub per_weekday: [usize; 7],
    pub per_hour: [usize; 24],
    /// Your most liked and reposted posts that aren't replies, best first.
    pub best: Vec<Kept>,
}

impl Stats {
    /// From the posts kept for you, as of `now`, in the time zone `tz`.
    pub fn of(kept: &[Kept], now: DateTime<Utc>, tz: FixedOffset) -> Self {
        let since = now - Duration::days(STATS_DAYS as i64);
        let today = now.with_timezone(&tz).date_naive();
        let mut out = Self {
            per_day: vec![0; STATS_DAYS],
            ..Self::default()
        };
        let mut best: Vec<&Kept> = Vec::new();
        for k in kept.iter().filter(|k| k.mine) {
            let Ok(at) = DateTime::parse_from_rfc3339(&k.created) else {
                continue;
            };
            if at < since || at > now {
                continue;
            }
            let local = at.with_timezone(&tz);
            if k.reply {
                out.replies += 1;
            } else {
                out.posts += 1;
                best.push(k);
            }
            out.likes += k.likes;
            out.reposts += k.reposts;
            out.replies_got += k.replies;
            let ago = (today - local.date_naive()).num_days();
            if let Some(day) = usize::try_from(ago)
                .ok()
                .and_then(|ago| STATS_DAYS.checked_sub(ago + 1))
            {
                out.per_day[day] += 1;
            }
            out.per_weekday[local.weekday().num_days_from_monday() as usize] += 1;
            out.per_hour[local.hour() as usize] += 1;
        }
        best.sort_by_key(|k| std::cmp::Reverse(k.likes + k.reposts * 2));
        out.best = best
            .into_iter()
            .filter(|k| k.likes + k.reposts > 0)
            .take(3)
            .cloned()
            .collect();
        out
    }

    /// The hour of day you post most, if you posted.
    pub fn busiest_hour(&self) -> Option<usize> {
        busiest(&self.per_hour)
    }

    /// The weekday you post most (0 is Monday), if you posted.
    pub fn busiest_weekday(&self) -> Option<usize> {
        busiest(&self.per_weekday)
    }
}

fn busiest(counts: &[usize]) -> Option<usize> {
    let (i, n) = counts
        .iter()
        .enumerate()
        .max_by_key(|(i, n)| (**n, std::cmp::Reverse(*i)))?;
    (*n > 0).then_some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mine(created: &str, reply: bool, likes: u64, reposts: u64) -> Kept {
        Kept {
            uri: format!("at://did:plc:me/app.bsky.feed.post/{created}").into(),
            did: "did:plc:me".into(),
            handle: "me.test".into(),
            name: String::new(),
            avatar: None,
            text: created.into(),
            created: created.into(),
            seen: created.into(),
            links: Vec::new(),
            card: None,
            images: Vec::new(),
            liked: false,
            saved: false,
            mine: true,
            reply,
            likes,
            reposts,
            replies: 1,
        }
    }

    #[test]
    fn counts_the_last_thirty_days_of_your_posts() {
        let now = "2026-10-08T12:00:00Z".parse().unwrap();
        let utc = FixedOffset::east_opt(0).unwrap();
        let mut someone = mine("2026-10-08T09:00:00Z", false, 100, 0);
        someone.mine = false;
        let kept = [
            mine("2026-10-08T09:00:00Z", false, 5, 1), // Thursday
            mine("2026-10-07T09:30:00Z", false, 20, 3),
            mine("2026-10-07T21:00:00Z", true, 1, 0),
            mine("2026-10-01T09:00:00Z", false, 0, 0),
            mine("2026-08-01T09:00:00Z", false, 999, 9), // too old
            someone,
        ];
        let s = Stats::of(&kept, now, utc);
        assert_eq!((s.posts, s.replies), (3, 1));
        assert_eq!((s.likes, s.reposts, s.replies_got), (26, 4, 4));
        assert_eq!(s.per_day.len(), STATS_DAYS);
        assert_eq!(s.per_day[STATS_DAYS - 1], 1, "today");
        assert_eq!(s.per_day[STATS_DAYS - 2], 2, "yesterday");
        assert_eq!(s.busiest_hour(), Some(9));
        assert_eq!(s.busiest_weekday(), Some(2), "Wednesday");
        let best: Vec<u64> = s.best.iter().map(|k| k.likes).collect();
        assert_eq!(best, [20, 5], "replies and posts nobody liked left out");
        assert_eq!(Stats::of(&[], now, utc).busiest_hour(), None);
    }
}
