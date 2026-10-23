//! What switched-on integrations read about links from their services: asked for
//! once a card is drawn, and kept for as long as the integration says its answer
//! holds. A failed answer is kept as long too, so a refusing service isn't asked
//! again for every frame.

use super::{App, Message};
use crate::api::{self, ApiError};
use crate::integrations::{self, Details};
use iced::Task;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Links kept at most; past this, answers that have run out are forgotten first.
const MAX_ENTRIES: usize = 2000;
/// How long a link whose integration reads nothing more is left alone.
const NOTHING_TO_ASK: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone)]
pub enum Msg {
    /// The service answered about this link.
    Read(String, Result<Vec<u8>, ApiError>),
}

enum Entry {
    /// Being asked about, with what was known before, if anything.
    Asking(Option<Details>),
    /// The answer, or `None` when there was none, and when it runs out.
    Known(Option<Details>, Instant),
}

#[derive(Default)]
pub(super) struct Cards {
    entries: HashMap<String, Entry>,
    /// Links drawn without an answer, to ask about on the next update.
    wanted: RefCell<HashSet<String>>,
}

impl Cards {
    /// What is known about `link`. A link not asked about yet, or whose answer has
    /// run out, is queued to be asked about.
    pub(super) fn get(&self, link: &str) -> Option<&Details> {
        match self.entries.get(link) {
            Some(Entry::Asking(d)) => d.as_ref(),
            Some(Entry::Known(d, until)) => {
                if *until <= Instant::now() {
                    self.wanted.borrow_mut().insert(link.to_string());
                }
                d.as_ref()
            }
            None => {
                self.wanted.borrow_mut().insert(link.to_string());
                None
            }
        }
    }

    /// Whether links are waiting to be asked about.
    pub(super) fn waiting(&self) -> bool {
        !self.wanted.borrow().is_empty()
    }

    fn take_wanted(&mut self) -> Vec<String> {
        self.wanted.get_mut().drain().collect()
    }

    fn forget_some(&mut self) {
        if self.entries.len() < MAX_ENTRIES {
            return;
        }
        let now = Instant::now();
        self.entries
            .retain(|_, e| !matches!(e, Entry::Known(_, until) if *until <= now));
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.clear();
        }
    }
}

impl App {
    pub(super) fn cards_update(&mut self, msg: Msg) -> Task<Message> {
        let Msg::Read(link, result) = msg;
        let Some(lookup) = integrations::lookup(&self.integrations, &self.settings, &link) else {
            self.cards.entries.remove(&link);
            return Task::none();
        };
        let before = match self.cards.entries.remove(&link) {
            Some(Entry::Known(d, _) | Entry::Asking(d)) => d,
            None => None,
        };
        let details = result
            .ok()
            .and_then(|body| {
                integrations::details(&self.integrations, &self.settings, &link, &body)
            })
            .or(before);
        let picture = details.as_ref().and_then(|d| d.thumbnail_url.clone());
        self.cards
            .entries
            .insert(link, Entry::Known(details, Instant::now() + lookup.keep));
        self.request_images(picture)
    }

    /// Asks the services about the links drawn without an answer. With the data saver
    /// on, nothing is asked.
    pub(super) fn fetch_cards(&mut self) -> Task<Message> {
        let wanted = self.cards.take_wanted();
        if wanted.is_empty() || self.settings.data_saver {
            return Task::none();
        }
        self.cards.forget_some();
        let mut tasks = Vec::new();
        for link in wanted {
            let Some(lookup) = integrations::lookup(&self.integrations, &self.settings, &link)
            else {
                let until = Instant::now() + NOTHING_TO_ASK;
                self.cards.entries.insert(link, Entry::Known(None, until));
                continue;
            };
            let before = match self.cards.entries.remove(&link) {
                Some(Entry::Known(d, _) | Entry::Asking(d)) => d,
                None => None,
            };
            self.cards
                .entries
                .insert(link.clone(), Entry::Asking(before));
            tasks.push(Task::perform(
                async move { api::fetch_bytes(&lookup.url).await },
                move |r| Message::Cards(Msg::Read(link.clone(), r)),
            ));
        }
        Task::batch(tasks)
    }

    /// What the integration showing `link` has read about it from its service.
    pub(crate) fn card_details(&self, link: &str) -> Option<&Details> {
        self.cards.get(link)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_wanted_until_asked_and_again_once_run_out() {
        let mut c = Cards::default();
        assert_eq!(c.get("l"), None);
        assert!(c.waiting());
        assert_eq!(c.take_wanted(), ["l"]);
        c.entries.insert("l".into(), Entry::Asking(None));
        c.get("l");
        assert!(!c.waiting(), "already being asked");
        let d = Details {
            title: Some("T".into()),
            ..Details::default()
        };
        let later = Instant::now() + Duration::from_secs(60);
        c.entries
            .insert("l".into(), Entry::Known(Some(d.clone()), later));
        assert_eq!(c.get("l"), Some(&d));
        assert!(!c.waiting());
        c.entries
            .insert("l".into(), Entry::Known(Some(d.clone()), Instant::now()));
        assert_eq!(c.get("l"), Some(&d), "kept while asked again");
        assert!(c.waiting());
    }

    #[test]
    fn answers_that_ran_out_are_forgotten_first() {
        let mut c = Cards::default();
        let later = Instant::now() + Duration::from_secs(60);
        for i in 0..MAX_ENTRIES {
            let until = if i == 0 { later } else { Instant::now() };
            c.entries.insert(i.to_string(), Entry::Known(None, until));
        }
        c.forget_some();
        assert_eq!(c.entries.len(), 1);
        assert!(c.entries.contains_key("0"));
    }
}
