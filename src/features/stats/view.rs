//! The stats screen: totals, a month of posting as bars, when you post, and your
//! best posts.

use super::Screen;
use crate::components::{self as ui, eyebrow, faint};
use crate::model::{Kept, STATS_DAYS, Stats};
use crate::theme::{self, gap, size};
use iced::widget::{Column, Row, column, container, row, space, text};
use iced::{Alignment, Element, Length};

/// Height of the tallest bar in the month chart.
const BAR_HEIGHT: f32 = 64.0;
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// What the stats screen needs from the app around it.
pub trait Host<M> {
    /// One of your kept posts, drawn as on the Offline reading screen.
    fn kept<'a>(&'a self, post: &'a Kept) -> Element<'a, M>;
}

pub struct View<'a, H> {
    pub screen: &'a Screen,
    pub stats: &'a Stats,
    pub host: &'a H,
}

impl<'a, H> View<'a, H> {
    pub fn view<M: Clone + 'a>(self) -> Element<'a, M>
    where
        H: Host<M>,
    {
        let s = self.stats;
        let mut col = Column::new().spacing(gap::M).padding([gap::S, gap::XL]);
        col = col.push(faint(if self.screen.loading {
            "Reading your latest posts…".to_string()
        } else if let Some(e) = &self.screen.error {
            format!("Couldn't read your latest posts, so these may be out of date: {e}")
        } else {
            format!(
                "The last {STATS_DAYS} days, worked out on this computer from your posts. Nothing is sent anywhere."
            )
        }));
        if s.posts + s.replies == 0 {
            return col
                .push(ui::muted("You haven't posted in the last month."))
                .into();
        }
        col = col.push(
            row![
                number(s.posts, "posts"),
                number(s.replies, "replies"),
                number(s.likes, "likes"),
                number(s.reposts, "reposts"),
                number(s.replies_got, "replies to you"),
            ]
            .spacing(gap::S),
        );
        col = col
            .push(space::vertical().height(gap::S))
            .push(eyebrow("Posts per day"))
            .push(ui::card(month(&s.per_day)).padding(gap::L));
        let mut when = Vec::new();
        if let Some(d) = s.busiest_weekday() {
            when.push(format!("You post most on {}s", WEEKDAYS[d]));
        }
        if let Some(h) = s.busiest_hour() {
            when.push(format!("around {h:02}:00"));
        }
        if !when.is_empty() {
            col = col.push(ui::muted(format!("{}.", when.join(", "))));
        }
        if !s.best.is_empty() {
            col = col
                .push(space::vertical().height(gap::S))
                .push(eyebrow("Your most liked posts"));
            let mut best = Column::new();
            for (i, k) in s.best.iter().enumerate() {
                if i > 0 {
                    best = best.push(ui::divider());
                }
                best = best.push(self.host.kept(k));
            }
            col = col.push(ui::card(best).padding(gap::XS));
        }
        col.into()
    }
}

fn number<'a, M: 'a>(n: impl std::fmt::Display, what: &'static str) -> Element<'a, M> {
    ui::card(
        column![
            text(n.to_string())
                .size(size::HEADING)
                .font(theme::SEMIBOLD),
            faint(what),
        ]
        .spacing(gap::XXS),
    )
    .padding([gap::M, gap::L])
    .width(Length::Fill)
    .into()
}

/// One bar per day, oldest on the left.
fn month<'a, M: 'a>(per_day: &[usize]) -> Element<'a, M> {
    let most = per_day.iter().copied().max().unwrap_or(0).max(1);
    let mut bars = Row::new().spacing(gap::HAIR).align_y(Alignment::End);
    for n in per_day {
        let height = (BAR_HEIGHT * *n as f32 / most as f32).max(2.0);
        bars = bars.push(
            container(space::vertical().height(height))
                .width(Length::Fill)
                .style(if *n > 0 { theme::bar } else { theme::bar_empty }),
        );
    }
    column![
        container(bars).height(BAR_HEIGHT).align_bottom(BAR_HEIGHT),
        row![
            faint(format!("{STATS_DAYS} days ago")),
            space::horizontal(),
            faint(format!("Most in a day: {most}")),
            space::horizontal(),
            faint("Today"),
        ],
    ]
    .spacing(gap::S)
    .into()
}
