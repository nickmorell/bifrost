//! The calls behind your Bluesky preferences.

use crate::api::{ApiError, Client};
use crate::model::{PrefEdit, Preferences};

/// Reading and changing the account's preferences.
pub trait PrefsApi: Clone + Send + Sync + 'static {
    fn preferences(&self) -> impl Future<Output = Result<Preferences, ApiError>> + Send;
    /// Applies `edits` to what the server has and gives back the result.
    fn edit_preferences(
        &self,
        edits: Vec<PrefEdit>,
    ) -> impl Future<Output = Result<Preferences, ApiError>> + Send;
}

impl PrefsApi for Client {
    fn preferences(&self) -> impl Future<Output = Result<Preferences, ApiError>> + Send {
        Client::preferences(self)
    }
    fn edit_preferences(
        &self,
        edits: Vec<PrefEdit>,
    ) -> impl Future<Output = Result<Preferences, ApiError>> + Send {
        Client::edit_preferences(self, edits)
    }
}
