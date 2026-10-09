//! The calls that take more than one step or choose between calls.

use super::state::{Editing, is_pack};
use crate::api::ApiError;
use crate::api::service::ListsApi;
use crate::model::{AtUri, Did, ListDetails, MAX_PACK_PEOPLE};

/// Blocks a moderation list, or unblocks it by deleting the block record. Returns the
/// block record now in place, if any.
pub(super) async fn block<A: ListsApi>(
    api: A,
    list: AtUri,
    current: Option<AtUri>,
) -> Result<Option<AtUri>, ApiError> {
    match current {
        Some(record) => api.delete(&record).await.map(|()| None),
        None => api.block_list(&list).await.map(Some),
    }
}

/// Follows everyone on a starter pack's list you don't follow yet. The pack's people
/// may not all be loaded, so this fetches them all first.
pub(super) async fn follow_all<A: ListsApi>(
    api: A,
    list: AtUri,
    me: Did,
) -> Result<Vec<(Did, AtUri)>, ApiError> {
    let people = api.list_members(&list, MAX_PACK_PEOPLE).await?;
    let dids: Vec<Did> = people
        .into_iter()
        .filter(|i| i.subject.viewer.following.is_none() && i.subject.did != me)
        .map(|i| i.subject.did)
        .collect();
    api.follow_all(&dids).await
}

/// Saves the editor: makes the list or pack, or changes it. Returns its URI.
pub(super) async fn save<A: ListsApi>(
    api: A,
    kind: Editing,
    details: ListDetails,
    feeds: Vec<AtUri>,
) -> Result<AtUri, ApiError> {
    match kind {
        Editing::NewList => api.create_list(&details).await,
        Editing::NewPack => api.create_starter_pack(&details, &[]).await,
        Editing::List(uri) => api.edit_list(&uri, &details).await.map(|()| uri),
        Editing::Pack { uri, .. } => api
            .edit_starter_pack(&uri, &details, &feeds)
            .await
            .map(|()| uri),
    }
}

/// Deletes a list, or a starter pack with the list that holds its people.
pub(super) async fn delete<A: ListsApi>(
    api: A,
    uri: AtUri,
    pack_list: Option<AtUri>,
) -> Result<(), ApiError> {
    if is_pack(&uri) {
        api.delete_starter_pack(&uri, pack_list.as_deref()).await
    } else {
        api.delete_list(&uri).await
    }
}

/// Puts someone on a list, or takes them off by deleting their entry. Returns their
/// entry now, if any.
pub(super) async fn toggle<A: ListsApi>(
    api: A,
    list: AtUri,
    did: Did,
    entry: Option<AtUri>,
) -> Result<Option<AtUri>, ApiError> {
    match entry {
        Some(record) => api.delete(&record).await.map(|()| None),
        None => api.add_to_list(&list, &did).await.map(Some),
    }
}
