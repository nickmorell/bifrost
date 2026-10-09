//! A fake Bluesky for the lists feature's tests.

use crate::api::ApiError;
use crate::api::service::ListsApi;
use crate::model::{
    AtUri, Author, Did, ListDetails, ListItem, ListPage, Membership, StarterPack, StarterPackBasic,
};
use std::sync::{Arc, Mutex};

use super::{LIST, PACK};

/// Bluesky as far as these tests need it: it notes every call and answers from what
/// it was given.
#[derive(Clone, Default)]
pub struct Fake {
    pub calls: Arc<Mutex<Vec<String>>>,
    pub members: Vec<ListItem>,
}

impl Fake {
    fn note(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    /// Notes the call and answers with `value`.
    fn answer<T: Send>(
        &self,
        call: String,
        value: T,
    ) -> impl Future<Output = Result<T, ApiError>> + Send + use<T> {
        self.note(call);
        async move { Ok(value) }
    }
}

impl ListsApi for Fake {
    fn list(
        &self,
        uri: &str,
        _: Option<String>,
    ) -> impl Future<Output = Result<ListPage, ApiError>> + Send {
        self.note(format!("list {uri}"));
        async { Err(ApiError::Other("not in the fake".into())) }
    }
    fn list_members(
        &self,
        uri: &str,
        _: usize,
    ) -> impl Future<Output = Result<Vec<ListItem>, ApiError>> + Send {
        self.answer(format!("members {uri}"), self.members.clone())
    }
    fn my_lists_for(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<Membership>, ApiError>> + Send {
        self.answer(format!("my lists for {actor}"), Vec::new())
    }
    fn create_list(&self, d: &ListDetails) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        self.answer(format!("create list {}", d.name), AtUri::from(LIST))
    }
    fn edit_list(
        &self,
        uri: &str,
        _: &ListDetails,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        self.answer(format!("edit list {uri}"), ())
    }
    fn delete_list(&self, uri: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        self.answer(format!("delete list {uri}"), ())
    }
    fn add_to_list(
        &self,
        list: &str,
        did: &str,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        self.answer(format!("add {did} to {list}"), AtUri::from("at://item"))
    }
    fn mute_list(
        &self,
        uri: &str,
        mute: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        self.answer(format!("mute {uri} {mute}"), ())
    }
    fn block_list(&self, uri: &str) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        self.answer(format!("block {uri}"), AtUri::from("at://block"))
    }
    fn starter_pack(
        &self,
        uri: &str,
    ) -> impl Future<Output = Result<StarterPack, ApiError>> + Send {
        self.note(format!("pack {uri}"));
        async { Err(ApiError::Other("not in the fake".into())) }
    }
    fn actor_starter_packs(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<StarterPackBasic>, ApiError>> + Send {
        self.answer(format!("packs of {actor}"), Vec::new())
    }
    fn create_starter_pack(
        &self,
        d: &ListDetails,
        _: &[Did],
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        self.answer(format!("create pack {}", d.name), AtUri::from(PACK))
    }
    fn edit_starter_pack(
        &self,
        uri: &str,
        _: &ListDetails,
        feeds: &[AtUri],
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        self.answer(format!("edit pack {uri} with {} feeds", feeds.len()), ())
    }
    fn delete_starter_pack(
        &self,
        uri: &str,
        list: Option<&str>,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        self.answer(format!("delete pack {uri} and {list:?}"), ())
    }
    fn follow_all(
        &self,
        dids: &[Did],
    ) -> impl Future<Output = Result<Vec<(Did, AtUri)>, ApiError>> + Send {
        let names: Vec<&str> = dids.iter().map(|d| &**d).collect();
        let done = dids
            .iter()
            .map(|d| (d.clone(), AtUri::from("at://follow")))
            .collect();
        self.answer(format!("follow {}", names.join(" ")), done)
    }
    fn typeahead(&self, q: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        self.answer(format!("typeahead {q}"), Vec::new())
    }
    fn delete(&self, uri: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        self.answer(format!("delete {uri}"), ())
    }
}
