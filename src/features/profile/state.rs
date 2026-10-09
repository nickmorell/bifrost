//! The profiles, feeds and lists loaded so far, and the "Edit profile" dialog.

use super::edit::Editor;
use super::tab::Tab;
use crate::model::{ActorViewer, FeedGenerator, ListInfo, Profile, StrongRef};
use std::collections::HashMap;

/// Something loaded for one account, or why it could not be.
type Loaded<T> = HashMap<String, Result<T, String>>;

/// Profiles by the DID or handle they were opened with, and the custom feeds and
/// lists each account made.
#[derive(Default)]
pub struct Profiles {
    pub(super) by_actor: Loaded<Profile>,
    pub(super) feeds: Loaded<Vec<FeedGenerator>>,
    pub(super) lists: Loaded<Vec<ListInfo>>,
    /// The "Edit profile" dialog, while it is open.
    pub(super) editor: Option<Editor>,
}

impl Profiles {
    /// The profile opened as `actor`, or why it could not load. `None` while loading.
    pub fn get(&self, actor: &str) -> Option<&Result<Profile, String>> {
        self.by_actor.get(actor)
    }

    /// The lists `actor` made, or why they could not load. `None` while loading.
    pub fn lists(&self, actor: &str) -> Option<&Result<Vec<ListInfo>, String>> {
        self.lists.get(actor)
    }

    /// The "Edit profile" dialog, while it is open.
    pub fn editor(&self) -> Option<&Editor> {
        self.editor.as_ref()
    }

    /// Forgets a profile that failed to load, so opening it tries again.
    pub fn forget_failed(&mut self, actor: &str) {
        if matches!(self.by_actor.get(actor), Some(Err(_))) {
            self.by_actor.remove(actor);
        }
    }

    /// Forgets the profile opened as `actor` and what `tab` shows of it, so both
    /// load again.
    pub fn forget(&mut self, actor: &str, tab: Tab) {
        self.by_actor.remove(actor);
        match tab {
            Tab::Feeds => _ = self.feeds.remove(actor),
            Tab::Lists => _ = self.lists.remove(actor),
            _ => {}
        }
    }

    /// Forgets the lists `actor` made, so they load again.
    pub fn forget_lists(&mut self, actor: &str) {
        self.lists.remove(actor);
    }

    /// Forgets every copy of the profile of `did`, however it was opened.
    pub fn forget_account(&mut self, did: &str) {
        self.by_actor
            .retain(|k, p| k != did && !matches!(p, Ok(p) if p.did == did));
    }

    /// Forgets everything, as switching accounts does.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Once more than `max` accounts are loaded, keeps only the ones `keep` wants.
    pub fn trim(&mut self, max: usize, keep: impl Fn(&str) -> bool) {
        if self.by_actor.len() > max {
            self.by_actor.retain(|did, _| keep(did));
        }
        if self.feeds.len() > max {
            self.feeds.retain(|did, _| keep(did));
        }
        if self.lists.len() > max {
            self.lists.retain(|did, _| keep(did));
        }
    }

    /// How you relate to the account `did` (following, muted, ...), if it is loaded.
    pub fn viewer(&self, did: &str) -> Option<&ActorViewer> {
        if let Some(Ok(p)) = self.by_actor.get(did) {
            return Some(&p.viewer);
        }
        self.loaded().find(|p| p.did == did).map(|p| &p.viewer)
    }

    /// Applies `f` to how you relate to the account `did`, in every loaded copy.
    pub fn with_actor(&mut self, did: &str, f: &mut impl FnMut(&mut ActorViewer)) {
        for p in self.loaded_mut().filter(|p| p.did == did) {
            f(&mut p.viewer);
        }
    }

    /// Shows `pin` as the post pinned to the profile of `did`.
    pub fn set_pinned(&mut self, did: &str, pin: Option<&StrongRef>) {
        for p in self.loaded_mut().filter(|p| p.did == did) {
            p.pinned_post = pin.cloned();
        }
    }

    fn loaded(&self) -> impl Iterator<Item = &Profile> {
        self.by_actor.values().filter_map(|p| p.as_ref().ok())
    }

    fn loaded_mut(&mut self) -> impl Iterator<Item = &mut Profile> {
        self.by_actor.values_mut().filter_map(|p| p.as_mut().ok())
    }
}
