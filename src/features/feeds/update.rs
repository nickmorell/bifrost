//! Keeping the Home tabs in step with your saved feeds, and loading the Feeds screen.

use super::state::UNNAMED;
use super::{Event, Feeds, Msg, Out, Tab, load};
use crate::api::service::FeedsApi;
use crate::features::Update;
use crate::model::{FeedSource, SavedFeed};
use iced::Task;
use std::collections::HashMap;

impl Feeds {
    /// `saved` is your saved feeds, from your preferences.
    pub fn update(&mut self, msg: Msg, saved: &[SavedFeed], api: &impl FeedsApi) -> Out {
        match msg {
            Msg::Sync => self.sync(saved, api),
            Msg::Reset => {
                self.tabs.clear();
                self.active = 0;
                self.sync(saved, api)
            }
            Msg::Names(names) => {
                let current = self.active().map(|t| t.source.clone());
                for (source, name) in names {
                    match name {
                        Some(name) => {
                            for tab in self.tabs.iter_mut().filter(|t| t.source == source) {
                                tab.name.clone_from(&name);
                            }
                        }
                        // A pinned feed that was deleted: drop its tab instead of showing
                        // an error every time it is opened.
                        None => self.tabs.retain(|t| t.source != source),
                    }
                }
                self.keep_open(current);
                Update::none()
            }
            Msg::Load { force } => {
                let missing: Vec<FeedSource> = saved
                    .iter()
                    .map(|f| f.source.clone())
                    .filter(|s| {
                        *s != FeedSource::Following && (force || !self.info.contains_key(s))
                    })
                    .collect();
                let api2 = api.clone();
                let info =
                    Task::perform(async move { load::info(&api2, missing).await }, Msg::Info);
                Update::task(Task::batch([info, self.suggest(force, api)]))
            }
            Msg::Suggest { force } => Update::task(self.suggest(force, api)),
            Msg::Info(info) => {
                let urls: Vec<String> = info.iter().filter_map(|(_, _, a)| a.clone()).collect();
                for (source, name, avatar) in info {
                    self.info.insert(source, (name, avatar));
                }
                Update::event(Event::Pictures(urls))
            }
            Msg::Suggested(Ok(feeds)) => {
                let urls: Vec<String> = feeds.iter().filter_map(|f| f.avatar.clone()).collect();
                for f in &feeds {
                    self.info.insert(
                        FeedSource::Generator(f.uri.clone()),
                        (f.display_name.clone(), f.avatar.clone()),
                    );
                }
                self.suggested = Some(Ok(feeds));
                Update::event(Event::Pictures(urls))
            }
            Msg::Suggested(Err(e)) => {
                self.suggested = Some(Err(e.friendly()));
                Update::none()
            }
            Msg::Select(i) => {
                let i = i.min(self.tabs.len().saturating_sub(1));
                if i == self.active {
                    // Clicking the tab you are on shows what is new, like the official app.
                    return Update::event(Event::ShowNew);
                }
                self.active = i;
                Update::event(Event::Selected)
            }
            Msg::Edit(edit) => Update::event(Event::Edit(edit)),
            Msg::Open(source, name) => Update::event(Event::Open(source, name)),
            Msg::Home => Update::event(Event::Home),
            Msg::Discover => Update::event(Event::Discover),
            Msg::Retry => self.update(Msg::Load { force: false }, saved, api),
            Msg::SignInAgain => Update::event(Event::SignInAgain),
        }
    }

    fn suggest(&self, force: bool, api: &impl FeedsApi) -> Task<Msg> {
        if !force && self.suggested.is_some() {
            return Task::none();
        }
        let api = api.clone();
        Task::perform(async move { api.suggested_feeds().await }, Msg::Suggested)
    }

    /// Rebuilds the Home tabs from the pinned feeds, keeping the open tab, and looks
    /// up names it does not know yet.
    fn sync(&mut self, saved: &[SavedFeed], api: &impl FeedsApi) -> Out {
        let current = self.active().map(|t| t.source.clone());
        let known: HashMap<FeedSource, String> =
            self.tabs.drain(..).map(|t| (t.source, t.name)).collect();
        self.tabs = saved
            .iter()
            .filter(|f| f.pinned || f.source == FeedSource::Following)
            .map(|f| Tab {
                name: match &f.source {
                    FeedSource::Following => "Following".into(),
                    s => known
                        .get(s)
                        .cloned()
                        .or_else(|| self.name(s).map(str::to_string))
                        .unwrap_or_else(|| UNNAMED.into()),
                },
                source: f.source.clone(),
            })
            .collect();
        self.keep_open(current);
        let unnamed: Vec<FeedSource> = self
            .tabs
            .iter()
            .filter(|t| t.name == UNNAMED)
            .map(|t| t.source.clone())
            .collect();
        if unnamed.is_empty() {
            return Update::none();
        }
        let api = api.clone();
        Update::task(Task::perform(
            async move { load::names(&api, unnamed).await },
            Msg::Names,
        ))
    }

    /// Keeps `current` open if it is still a tab, or opens the first.
    fn keep_open(&mut self, current: Option<FeedSource>) {
        self.active = current
            .and_then(|c| self.tabs.iter().position(|t| t.source == c))
            .unwrap_or(0);
    }
}
