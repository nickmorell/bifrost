//! Translating posts: through a translation service, or by opening Google
//! Translate in the browser.

use crate::api::ApiError;
use crate::api::service::TranslateApi;
use crate::model::{TranslateService, Translation};
use crate::storage::secrets::{KeyringStore, SecretStore, StoreError};
use std::sync::Arc;

mod services;

/// The name a service's API key is saved under.
fn secret_name(service: TranslateService) -> String {
    format!("translate:{}", service.name().to_lowercase())
}

/// Translates through the chosen service, with API keys kept in a [`SecretStore`].
#[derive(Clone)]
pub struct Translator {
    secrets: Arc<dyn SecretStore>,
}

impl Default for Translator {
    /// Keys in the OS credential store.
    fn default() -> Self {
        Self::new(Arc::new(KeyringStore::default()))
    }
}

impl Translator {
    pub fn new(secrets: Arc<dyn SecretStore>) -> Self {
        Self { secrets }
    }

    /// The API key saved for a service, if any. Reads the store, which can block.
    fn key_now(&self, service: TranslateService) -> Option<String> {
        self.secrets
            .get(&secret_name(service))
            .ok()
            .flatten()
            .and_then(|b| String::from_utf8(b).ok())
            .filter(|k| !k.is_empty())
    }

    /// Saves a key, or removes it when empty. Writes the store, which can block.
    fn set_key_now(&self, service: TranslateService, key: &str) -> Result<(), StoreError> {
        let name = secret_name(service);
        if key.trim().is_empty() {
            self.secrets.delete(&name)
        } else {
            self.secrets.set(&name, key.trim().as_bytes())
        }
    }

    /// The key off the window's thread.
    async fn key(&self, service: TranslateService) -> Option<String> {
        let me = self.clone();
        tokio::task::spawn_blocking(move || me.key_now(service))
            .await
            .ok()
            .flatten()
    }
}

impl TranslateApi for Translator {
    async fn translate(
        &self,
        service: TranslateService,
        text: String,
        from: Option<String>,
        to: String,
        server: String,
    ) -> Result<Translation, ApiError> {
        let net = crate::net::shared();
        match service {
            TranslateService::MyMemory => {
                services::MyMemory(net)
                    .translate(&text, from.as_deref(), &to)
                    .await
            }
            TranslateService::DeepL => {
                let key = self.key(TranslateService::DeepL).await.ok_or_else(|| {
                    ApiError::Other("Add your DeepL API key in Settings first.".into())
                })?;
                services::DeepL { net, key }.translate(&text, &to).await
            }
            TranslateService::LibreTranslate => {
                let server = server.trim().trim_end_matches('/');
                if server.is_empty() {
                    return Err(ApiError::Other(
                        "Add your LibreTranslate server's address in Settings first.".into(),
                    ));
                }
                if let Some(why) = crate::api::insecure_server(server) {
                    return Err(ApiError::Other(why.replace("password", "API key")));
                }
                let libre = services::Libre {
                    net,
                    server,
                    key: self.key(TranslateService::LibreTranslate).await,
                };
                libre.translate(&text, &to).await
            }
            TranslateService::Browser => {
                Err(ApiError::Other("Translations open in the browser.".into()))
            }
        }
    }

    async fn has_key(&self, service: TranslateService) -> bool {
        self.key(service).await.is_some()
    }

    async fn set_key(&self, service: TranslateService, key: String) -> Result<(), ApiError> {
        let me = self.clone();
        tokio::task::spawn_blocking(move || me.set_key_now(service, &key))
            .await
            .map_err(|e| ApiError::Other(e.to_string()))?
            .map_err(|e| ApiError::Other(e.to_string()))
    }
}

/// MyMemory takes at most 500 bytes at a time.
const MY_MEMORY_MAX: usize = 480;

/// Splits text into pieces of at most `max` bytes, at line or sentence ends where
/// it can and between characters otherwise.
fn chunks(text: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while rest.len() > max {
        let mut cut = max;
        while !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        let head = &rest[..cut];
        let at = head
            .rfind('\n')
            .or_else(|| {
                head.rfind(['.', '!', '?', '。', '！', '？'])
                    .map(|i| i + head[i..].chars().next().map_or(1, char::len_utf8))
            })
            .or_else(|| head.rfind(' '))
            .filter(|&i| i > 0)
            .unwrap_or(cut);
        out.push(rest[..at].to_string());
        rest = &rest[at..];
    }
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_long_text() {
        let s = "Hello there. ".repeat(60);
        let parts = chunks(&s, 100);
        assert!(parts.iter().all(|p| p.len() <= 100));
        assert_eq!(parts.concat(), s);
        let cjk = "日本語の文章です。".repeat(40);
        let parts = chunks(&cjk, 100);
        assert!(parts.iter().all(|p| p.len() <= 100));
        assert_eq!(parts.concat(), cjk);
        assert_eq!(chunks("short", 100), vec!["short".to_string()]);
    }

    #[test]
    fn keys_are_kept_in_the_store_and_empty_removes_them() {
        let store = Arc::new(crate::storage::secrets::MemoryStore::default());
        let t = Translator::new(store.clone());
        assert_eq!(t.key_now(TranslateService::DeepL), None);
        t.set_key_now(TranslateService::DeepL, "  abc  ").unwrap();
        assert_eq!(t.key_now(TranslateService::DeepL).as_deref(), Some("abc"));
        assert_eq!(
            store.get("translate:deepl").unwrap().as_deref(),
            Some(&b"abc"[..])
        );
        t.set_key_now(TranslateService::DeepL, " ").unwrap();
        assert_eq!(t.key_now(TranslateService::DeepL), None);
    }
}
