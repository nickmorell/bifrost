//! The calls behind the Privacy settings.

use crate::api::{ApiError, Client};
use crate::model::Privacy;

/// What the Privacy settings read and change.
pub trait PrivacyApi: Clone + Send + Sync + 'static {
    fn privacy(&self) -> impl Future<Output = Result<Privacy, ApiError>> + Send;
    fn set_hide_from_discover(
        &self,
        hide: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn set_hide_logged_out(&self, hide: bool) -> impl Future<Output = Result<(), ApiError>> + Send;
}

impl PrivacyApi for Client {
    fn privacy(&self) -> impl Future<Output = Result<Privacy, ApiError>> + Send {
        Client::privacy(self)
    }
    fn set_hide_from_discover(
        &self,
        hide: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::set_hide_from_discover(self, hide)
    }
    fn set_hide_logged_out(&self, hide: bool) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::set_hide_logged_out(self, hide)
    }
}
