//! The calls behind the ATstore screens: the whole directory, and one listing's page.

use crate::api::atstore::Directory;
use crate::api::{ApiError, Client};
use crate::model::{AtUri, Catalog, ListingDetail};
use crate::net::shared;

/// What the ATstore screens ask of atstore.fyi. Calls return futures that can move to
/// another thread, so they can run as iced tasks.
pub trait AtstoreApi: Clone + Send + Sync + 'static {
    /// Every listing, read now.
    fn catalog(&self) -> impl Future<Output = Result<Catalog, ApiError>> + Send;

    /// One listing's page.
    fn listing(&self, uri: &AtUri) -> impl Future<Output = Result<ListingDetail, ApiError>> + Send;
}

impl AtstoreApi for Client {
    fn catalog(&self) -> impl Future<Output = Result<Catalog, ApiError>> + Send {
        Box::pin(async move {
            let listings = Directory(shared()).all().await?;
            Ok(Catalog {
                fetched: chrono::Utc::now().to_rfc3339(),
                listings,
                ..Catalog::default()
            })
        })
    }

    fn listing(&self, uri: &AtUri) -> impl Future<Output = Result<ListingDetail, ApiError>> + Send {
        let uri = uri.clone();
        Box::pin(async move { Directory(shared()).listing(&uri).await })
    }
}
