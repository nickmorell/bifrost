//! The lists of accounts loaded so far.

use crate::model::{Author, PeopleKind};
use std::collections::HashMap;

/// Every list of accounts loaded, by actor and which list.
#[derive(Debug, Default)]
pub struct People {
    pub(super) lists: HashMap<(String, PeopleKind), List>,
}

/// A page-by-page list of accounts, such as someone's followers.
#[derive(Debug, Default)]
pub(super) struct List {
    pub(super) items: Vec<Author>,
    pub(super) cursor: Option<String>,
    pub(super) loading: bool,
    pub(super) error: Option<String>,
}

impl People {
    pub fn contains(&self, actor: &str, kind: PeopleKind) -> bool {
        self.lists.contains_key(&(actor.to_string(), kind))
    }

    pub fn count(&self) -> usize {
        self.lists.len()
    }

    pub fn clear(&mut self) {
        self.lists.clear();
    }

    /// Keeps only the lists `keep` says yes to.
    pub fn retain(&mut self, mut keep: impl FnMut(&str, PeopleKind) -> bool) {
        self.lists.retain(|(actor, kind), _| keep(actor, *kind));
    }

    /// Every account in every list, to change in place.
    pub fn authors_mut(&mut self) -> impl Iterator<Item = &mut Author> {
        self.lists.values_mut().flat_map(|l| l.items.iter_mut())
    }
}
