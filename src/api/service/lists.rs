//! The calls behind lists and starter packs.

use crate::api::{ApiError, Client};
use crate::model::{
    AtUri, Author, Did, ListDetails, ListItem, ListPage, Membership, StarterPack, StarterPackBasic,
};

/// Reading, making and changing lists and starter packs. Each call returns a future
/// that can move to another thread, so it can run as an iced task.
pub trait ListsApi: Clone + Send + Sync + 'static {
    /// A list's details and a page of its people.
    fn list(
        &self,
        uri: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<ListPage, ApiError>> + Send;
    /// Up to `max` of a list's people, across pages.
    fn list_members(
        &self,
        uri: &str,
        max: usize,
    ) -> impl Future<Output = Result<Vec<ListItem>, ApiError>> + Send;
    /// Your lists, each with this person's entry on it if they are on it.
    fn my_lists_for(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<Membership>, ApiError>> + Send;
    fn create_list(
        &self,
        details: &ListDetails,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    fn edit_list(
        &self,
        uri: &str,
        details: &ListDetails,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn delete_list(&self, uri: &str) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// Puts someone on a list and returns their entry's URI.
    fn add_to_list(
        &self,
        list: &str,
        did: &str,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    fn mute_list(&self, uri: &str, mute: bool)
    -> impl Future<Output = Result<(), ApiError>> + Send;
    /// Blocks everyone on a moderation list and returns the block's URI.
    fn block_list(&self, uri: &str) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    fn starter_pack(&self, uri: &str)
    -> impl Future<Output = Result<StarterPack, ApiError>> + Send;
    fn actor_starter_packs(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<StarterPackBasic>, ApiError>> + Send;
    fn create_starter_pack(
        &self,
        details: &ListDetails,
        people: &[Did],
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    fn edit_starter_pack(
        &self,
        uri: &str,
        details: &ListDetails,
        feeds: &[AtUri],
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn delete_starter_pack(
        &self,
        uri: &str,
        list: Option<&str>,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// Follows everyone given and returns each new follow's URI.
    fn follow_all(
        &self,
        dids: &[Did],
    ) -> impl Future<Output = Result<Vec<(Did, AtUri)>, ApiError>> + Send;
    /// People whose name or handle starts with `q`.
    fn typeahead(&self, q: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send;
    /// Deletes one of your records, such as a list entry or a list block.
    fn delete(&self, uri: &str) -> impl Future<Output = Result<(), ApiError>> + Send;
}

impl ListsApi for Client {
    fn list(
        &self,
        uri: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<ListPage, ApiError>> + Send {
        Client::list(self, uri, cursor)
    }
    fn list_members(
        &self,
        uri: &str,
        max: usize,
    ) -> impl Future<Output = Result<Vec<ListItem>, ApiError>> + Send {
        Client::list_members(self, uri, max)
    }
    fn my_lists_for(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<Membership>, ApiError>> + Send {
        Client::my_lists_for(self, actor)
    }
    fn create_list(
        &self,
        details: &ListDetails,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::create_list(self, details)
    }
    fn edit_list(
        &self,
        uri: &str,
        details: &ListDetails,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::edit_list(self, uri, details)
    }
    fn delete_list(&self, uri: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::delete_list(self, uri)
    }
    fn add_to_list(
        &self,
        list: &str,
        did: &str,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::add_to_list(self, list, did)
    }
    fn mute_list(
        &self,
        uri: &str,
        mute: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::mute_list(self, uri, mute)
    }
    fn block_list(&self, uri: &str) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::block_list(self, uri)
    }
    fn starter_pack(
        &self,
        uri: &str,
    ) -> impl Future<Output = Result<StarterPack, ApiError>> + Send {
        Client::starter_pack(self, uri)
    }
    fn actor_starter_packs(
        &self,
        actor: &str,
    ) -> impl Future<Output = Result<Vec<StarterPackBasic>, ApiError>> + Send {
        Client::actor_starter_packs(self, actor)
    }
    fn create_starter_pack(
        &self,
        details: &ListDetails,
        people: &[Did],
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::create_starter_pack(self, details, people)
    }
    fn edit_starter_pack(
        &self,
        uri: &str,
        details: &ListDetails,
        feeds: &[AtUri],
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::edit_starter_pack(self, uri, details, feeds)
    }
    fn delete_starter_pack(
        &self,
        uri: &str,
        list: Option<&str>,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::delete_starter_pack(self, uri, list)
    }
    fn follow_all(
        &self,
        dids: &[Did],
    ) -> impl Future<Output = Result<Vec<(Did, AtUri)>, ApiError>> + Send {
        Client::follow_all(self, dids)
    }
    fn typeahead(&self, q: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        Client::typeahead(self, q)
    }
    fn delete(&self, uri: &str) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::delete(self, uri)
    }
}
