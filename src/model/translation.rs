//! Translating posts: which service does it (chosen in Settings) and what comes back.

use serde::{Deserialize, Serialize};

/// Where translations come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranslateService {
    /// MyMemory's free service: no account needed.
    #[default]
    MyMemory,
    /// DeepL, with your own API key.
    DeepL,
    /// A LibreTranslate server, such as one you run yourself.
    LibreTranslate,
    /// Google Translate, opened in the browser.
    Browser,
}

impl TranslateService {
    pub const ALL: [TranslateService; 4] = [
        TranslateService::MyMemory,
        TranslateService::DeepL,
        TranslateService::LibreTranslate,
        TranslateService::Browser,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TranslateService::MyMemory => "MyMemory",
            TranslateService::DeepL => "DeepL",
            TranslateService::LibreTranslate => "LibreTranslate",
            TranslateService::Browser => "Browser",
        }
    }

    pub fn needs_key(self) -> bool {
        self == TranslateService::DeepL
    }
}

/// A finished translation.
#[derive(Debug, Clone, PartialEq)]
pub struct Translation {
    pub text: String,
    /// The language the post was in, as a lowercase code, when known.
    pub from: Option<String>,
}

/// Google Translate's page for some text.
pub fn browser_url(text: &str, to: &str) -> String {
    url::Url::parse_with_params(
        "https://translate.google.com/",
        [
            ("sl", "auto"),
            ("tl", to),
            ("text", text),
            ("op", "translate"),
        ],
    )
    .map_or_else(|_| "https://translate.google.com/".into(), String::from)
}

/// The base language of a tag such as "pt-BR".
pub fn base_lang(tag: &str) -> String {
    tag.split(['-', '_']).next().unwrap_or(tag).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_and_browser_links() {
        assert_eq!(base_lang("pt-BR"), "pt");
        assert_eq!(base_lang("EN"), "en");
        let url = browser_url("hola & adiós", "en");
        assert!(url.starts_with("https://translate.google.com/?sl=auto&tl=en&text=hola"));
        assert!(!url.contains(' '));
    }
}
