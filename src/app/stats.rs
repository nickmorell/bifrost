//! Your stats (`features::stats`): bringing your posts up to date, keeping them
//! with the others on this computer, and working out the numbers.

use super::{App, Message};
use crate::features::stats::{Event, Host, Screen, View};
use crate::model::{Kept, Stats};
use chrono::Offset;
use iced::{Element, Task};

pub use crate::features::stats::Msg;

/// The stats screen and the numbers last worked out.
#[derive(Default)]
pub(super) struct Mine {
    screen: Screen,
    figures: Stats,
}

impl App {
    pub(super) fn stats_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.stats.screen.update(msg, &self.client);
        for Event::Posts(posts) in out.events {
            let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            let me = self.my_did.as_str();
            let kept: Vec<Kept> = posts.iter().map(|p| Kept::of(p, me, &at)).collect();
            self.archive.keep(kept);
            self.archive.save(false);
            self.count_stats();
        }
        out.task.map(Message::Stats)
    }

    /// Opening the screen: the numbers from what is kept, then fresher posts.
    pub(super) fn stats_enter(&mut self) -> Task<Message> {
        self.count_stats();
        let me = self.my_did.to_string();
        self.stats_update(Msg::Load(me))
    }

    fn count_stats(&mut self) {
        let tz = chrono::Local::now().offset().fix();
        self.stats.figures = Stats::of(self.archive.posts(), chrono::Utc::now(), tz);
    }

    pub(super) fn stats_view(&self) -> Element<'_, Message> {
        self.scroll(
            View {
                screen: &self.stats.screen,
                stats: &self.stats.figures,
                host: self,
            }
            .view(),
        )
    }
}

impl Host<Message> for App {
    fn kept<'a>(&'a self, post: &'a Kept) -> Element<'a, Message> {
        self.kept_view(post)
    }
}
