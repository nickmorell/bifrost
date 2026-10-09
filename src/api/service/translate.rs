//! What translating posts needs: a service to send text to, and the API keys kept
//! for it in the system's secure storage.

use crate::api::ApiError;
use crate::model::{TranslateService, Translation};

/// Translating text and keeping API keys. Each call returns a future that can move to
/// another thread; reading and writing keys never runs on the window's thread.
pub trait TranslateApi: Clone + Send + Sync + 'static {
    /// Translates `text` into `to` (a code such as "en"). `from` is the post's own
    /// language tag, when it has one; `server` is the LibreTranslate server.
    fn translate(
        &self,
        service: TranslateService,
        text: String,
        from: Option<String>,
        to: String,
        server: String,
    ) -> impl Future<Output = Result<Translation, ApiError>> + Send;
    /// Whether an API key is saved for the service.
    fn has_key(&self, service: TranslateService) -> impl Future<Output = bool> + Send;
    /// Saves the service's API key; an empty key removes it.
    fn set_key(
        &self,
        service: TranslateService,
        key: String,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
}
