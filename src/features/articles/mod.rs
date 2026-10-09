//! Long-form articles: the reader, your Standard.site subscriptions, and the
//! Articles screen listing what the publications you follow published lately.
//!
//! Articles are read from their authors' own servers. Subscribing writes a
//! Standard.site subscription record, so other apps see it too.

mod view;

#[cfg(test)]
mod tests;

pub use view::{List, Reader};

use crate::api::ApiError;
use crate::api::service::ArticlesApi;
use crate::features::Update;
use crate::model::{Article, ArticleRef, ArticleSummary, AtUri};
use iced::Task;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum Msg {
    /// Read your subscriptions again.
    Load,
    Loaded(Result<Vec<(AtUri, AtUri)>, ApiError>),
    /// Subscribe to, or unsubscribe from, a publication.
    Toggle(AtUri),
    Subscribed(AtUri, Result<AtUri, ApiError>),
    Unsubscribed(AtUri, AtUri, Result<(), ApiError>),
    /// Show an article in the reader.
    Open(ArticleRef),
    Opened(ArticleRef, Result<Article, ApiError>),
    /// Fetch the newest articles from your subscriptions.
    Refresh,
    Latest(Vec<ArticleSummary>),
    OpenUrl(String),
    /// Go to an article's page.
    Read(ArticleRef),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Toast(String),
    OpenUrl(String),
    /// Go to an article's page.
    Read(ArticleRef),
    Pictures(Vec<String>),
}

pub type Out = Update<Msg, Event>;

/// An article in the reader: loading, read, or failed.
#[derive(Debug, Default)]
pub enum Reading {
    #[default]
    Loading,
    Ready(Box<Article>),
    Failed(String),
}

#[derive(Debug, Default)]
pub struct Articles {
    /// Publication → your subscription record.
    subscribed: HashMap<AtUri, AtUri>,
    /// Publications being subscribed or unsubscribed.
    pending: HashSet<AtUri>,
    /// The article in the reader.
    reader: Option<(ArticleRef, Reading)>,
    /// The newest articles from your subscriptions, once fetched.
    latest: Option<Vec<ArticleSummary>>,
    refreshing: bool,
}

impl Articles {
    /// Whether you subscribe to `publication`, and whether that is changing now.
    pub fn subscription(&self, publication: &AtUri) -> (bool, bool) {
        (
            self.subscribed.contains_key(publication),
            self.pending.contains(publication),
        )
    }

    pub fn update(&mut self, msg: Msg, api: &impl ArticlesApi) -> Out {
        match msg {
            Msg::Load => {
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.subscriptions().await },
                    Msg::Loaded,
                ))
            }
            Msg::Loaded(Ok(subs)) => {
                self.subscribed = subs.into_iter().map(|(rec, publ)| (publ, rec)).collect();
                Update::none()
            }
            // Without the list every card offers Subscribe, which is still right to
            // press: a second subscription record does no harm.
            Msg::Loaded(Err(_)) => Update::none(),
            Msg::Toggle(publication) => self.toggle(publication, api),
            Msg::Subscribed(publication, result) => {
                self.pending.remove(&publication);
                match result {
                    Ok(rec) => {
                        self.subscribed.insert(publication, rec);
                        self.latest = None;
                        Update::event(Event::Toast("Subscribed.".into()))
                    }
                    Err(e) => failed("subscribe", &e),
                }
            }
            Msg::Unsubscribed(publication, rec, result) => {
                self.pending.remove(&publication);
                match result {
                    Ok(()) => {
                        self.latest = None;
                        Update::none()
                    }
                    Err(e) => {
                        self.subscribed.insert(publication, rec);
                        failed("unsubscribe", &e)
                    }
                }
            }
            Msg::Open(article) => {
                if matches!(&self.reader, Some((a, Reading::Ready(_))) if *a == article) {
                    return Update::none();
                }
                self.reader = Some((article.clone(), Reading::Loading));
                let api = api.clone();
                Update::task(Task::perform(
                    async move {
                        let r = api.article(&article).await;
                        (article, r)
                    },
                    |(a, r)| Msg::Opened(a, r),
                ))
            }
            Msg::Opened(article, result) => {
                let Some((current, reading)) = &mut self.reader else {
                    return Update::none();
                };
                if *current != article {
                    return Update::none();
                }
                match result {
                    Ok(a) => {
                        let pictures = a
                            .cover
                            .iter()
                            .chain(a.publication.as_ref().and_then(|p| p.icon.as_ref()))
                            .cloned()
                            .collect();
                        *reading = Reading::Ready(Box::new(a));
                        Update::event(Event::Pictures(pictures))
                    }
                    Err(e) => {
                        *reading = Reading::Failed(e.friendly());
                        Update::none()
                    }
                }
            }
            Msg::Refresh => {
                if self.refreshing {
                    return Update::none();
                }
                self.refreshing = true;
                let publications: Vec<AtUri> = self.subscribed.keys().cloned().collect();
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.latest(&publications).await },
                    Msg::Latest,
                ))
            }
            Msg::Latest(found) => {
                self.refreshing = false;
                self.latest = Some(found);
                Update::none()
            }
            Msg::OpenUrl(url) => Update::event(Event::OpenUrl(url)),
            Msg::Read(article) => Update::event(Event::Read(article)),
        }
    }

    /// The Articles screen needs its list fetched.
    pub fn stale(&self) -> bool {
        self.latest.is_none() && !self.refreshing
    }

    fn toggle(&mut self, publication: AtUri, api: &impl ArticlesApi) -> Out {
        if !self.pending.insert(publication.clone()) {
            return Update::none();
        }
        let api = api.clone();
        let task = match self.subscribed.remove(&publication) {
            Some(rec) => Task::perform(
                async move {
                    let r = api.unsubscribe(&rec).await;
                    (publication, rec, r)
                },
                |(p, rec, r)| Msg::Unsubscribed(p, rec, r),
            ),
            None => Task::perform(
                async move {
                    let r = api.subscribe(&publication).await;
                    (publication, r)
                },
                |(p, r)| Msg::Subscribed(p, r),
            ),
        };
        Update::task(task)
    }
}

fn failed(what: &str, e: &ApiError) -> Out {
    Update::event(Event::Toast(format!("Couldn't {what}: {}", e.friendly())))
}
