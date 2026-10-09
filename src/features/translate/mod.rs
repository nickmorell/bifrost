//! Translating posts inside BiFrost, and the Translation card in Settings.
//!
//! Translations go through [`TranslateApi`](crate::api::service::TranslateApi), which
//! also keeps the services' API keys. The app keeps which service, language and
//! server are chosen, and saves them when [`Event`]s ask.

mod update;
mod view;

#[cfg(test)]
mod tests;

pub use view::{settings, under_post};

use crate::api::ApiError;
use crate::model::{AtUri, PostView, TranslateService as Service, Translation, base_lang};
use std::collections::HashMap;

/// One post's translation.
#[derive(Debug, Clone)]
pub enum State {
    Loading,
    Done(Translation),
    Failed(String),
}

#[derive(Debug, Default)]
pub struct Translations {
    /// By post URI.
    posts: HashMap<AtUri, State>,
    /// The API key being typed in Settings.
    key_input: String,
    /// Whether DeepL and LibreTranslate have keys saved. Read when Settings opens.
    has_key: HashMap<Service, bool>,
}

/// The services that take an API key.
const KEYED: [Service; 2] = [Service::DeepL, Service::LibreTranslate];

impl Translations {
    /// Forgets every post's translation, as when the cache is trimmed.
    pub fn clear(&mut self) {
        self.posts.clear();
    }

    pub fn len(&self) -> usize {
        self.posts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.posts.is_empty()
    }
}

/// What is chosen in Settings.
#[derive(Debug, Clone)]
pub struct Ctx {
    pub service: Service,
    /// The language posts are translated into.
    pub to: String,
    /// The LibreTranslate server.
    pub server: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Translate {
        uri: AtUri,
        text: String,
        lang: Option<String>,
    },
    Done(AtUri, Result<Translation, ApiError>),
    Hide(AtUri),
    To(String),
    Service(Service),
    Server(String),
    KeyInput(String),
    SaveKey,
    KeySaved(Service, bool, Result<(), ApiError>),
    /// Settings opened: find out which services have keys.
    ReadKeys,
    KeysRead(Vec<(Service, bool)>),
    /// Remove every saved key.
    ForgetKeys,
    KeysForgotten(Result<(), ApiError>),
}

/// What the app has to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    OpenUrl(String),
    /// Save these choices.
    To(String),
    Service(Service),
    Server(String),
    Toast(String),
}

/// The message that translates a post.
pub fn translate(post: &PostView) -> Msg {
    Msg::Translate {
        uri: post.uri.clone(),
        text: post.record.text.clone(),
        lang: post.record.langs.first().map(|l| base_lang(l)),
    }
}

/// Whether a post is in another language than `to`, going by its language tag.
pub fn is_foreign(post: &PostView, to: &str) -> bool {
    !post.record.text.trim().is_empty()
        && post
            .record
            .langs
            .first()
            .is_some_and(|l| base_lang(l) != to)
}
