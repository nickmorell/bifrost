//! What the lists feature keeps: each list and pack seen, and its dialogs.

use crate::model::{
    AtUri, Author, Did, FeedSource, ListItem, ListView, Membership, StarterPack, StarterPackBasic,
};
use std::collections::{HashMap, HashSet};

/// The longest list or pack name Bluesky takes.
pub(super) const MAX_NAME: usize = 64;
/// The longest description Bluesky takes.
pub(super) const MAX_DESCRIPTION: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListTab {
    #[default]
    Posts,
    People,
}

/// One list as its screen shows it.
#[derive(Debug, Default)]
pub struct ListScreen {
    pub list: Option<Result<ListView, String>>,
    pub items: Vec<ListItem>,
    pub cursor: Option<String>,
    pub loading: bool,
    pub tab: ListTab,
    /// Finding people to add, on your own lists.
    pub query: String,
    pub(super) seq: u64,
    pub results: Vec<Author>,
    /// People being added or removed.
    pub busy: HashSet<Did>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Editing {
    NewList,
    NewPack,
    List(AtUri),
    Pack { uri: AtUri, list: Option<AtUri> },
}

/// The dialog for making or editing a list or starter pack.
#[derive(Debug)]
pub struct Editor {
    pub kind: Editing,
    pub name: String,
    pub description: String,
    pub moderation: bool,
    /// A starter pack's feeds.
    pub feeds: Vec<AtUri>,
    pub saving: bool,
    pub error: Option<String>,
}

impl Editor {
    pub(super) fn new(kind: Editing, name: String, description: String) -> Self {
        Self {
            kind,
            name,
            description,
            moderation: false,
            feeds: Vec::new(),
            saving: false,
            error: None,
        }
    }
}

/// "Add to lists" for one person.
#[derive(Debug)]
pub struct AddTo {
    pub did: Did,
    pub name: String,
    pub rows: Option<Result<Vec<Membership>, String>>,
    pub busy: HashSet<AtUri>,
}

#[derive(Debug, Default)]
pub struct Lists {
    pub screens: HashMap<AtUri, ListScreen>,
    pub packs: HashMap<AtUri, Result<StarterPack, String>>,
    pub actor_packs: HashMap<String, Result<Vec<StarterPackBasic>, String>>,
    /// The Lists screen is showing starter packs.
    pub packs_tab: bool,
    pub editor: Option<Editor>,
    pub add_to: Option<AddTo>,
    /// A list or pack waiting for "Delete?" to be confirmed.
    pub confirm_delete: Option<AtUri>,
    pub following_all: HashSet<AtUri>,
}

impl Lists {
    /// A list, once loaded.
    pub fn list(&self, uri: &str) -> Option<&ListView> {
        match self.screens.get(uri).and_then(|s| s.list.as_ref()) {
            Some(Ok(l)) => Some(l),
            _ => None,
        }
    }

    /// The timeline a list screen shows, when on its Posts tab.
    pub fn timeline(&self, uri: &AtUri) -> Option<FeedSource> {
        let s = self.screens.get(uri);
        let is_mod = self.list(uri).is_some_and(ListView::is_mod);
        (s.is_none_or(|s| s.tab == ListTab::Posts) && !is_mod)
            .then(|| FeedSource::List(uri.clone()))
    }

    /// The title bar says what kind of page this is; the name is in the header below.
    pub fn title(&self, uri: &str) -> String {
        match self.list(uri) {
            Some(l) if l.is_mod() => "Moderation list".into(),
            _ => "List".into(),
        }
    }

    pub(super) fn with_list(&mut self, uri: &str, f: impl FnOnce(&mut ListView)) {
        if let Some(Some(Ok(l))) = self.screens.get_mut(uri).map(|s| &mut s.list) {
            f(l);
        }
    }
}

pub(super) fn is_pack(uri: &str) -> bool {
    uri.contains("/app.bsky.graph.starterpack/")
}
