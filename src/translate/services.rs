//! The translation services, each a small [`ApiClient`] that only says where it is,
//! how it signs in and how its replies look.

use super::{MY_MEMORY_MAX, chunks};
use crate::api::ApiError;
use crate::api::client::{ApiClient, Problem, server_message, status_text};
use crate::model::Translation;
use crate::net::Transport;
use serde_json::{Value, json};

/// How every translation service's failures read.
fn service_error(problem: Problem) -> ApiError {
    ApiError::Other(match problem {
        Problem::Unreachable(e) => format!("Couldn't reach the translation service: {e}"),
        Problem::TooBig => "The translation service's reply was too large.".into(),
        Problem::Status(r) => match r.status {
            401 | 403 => "The translation service turned down the API key.".into(),
            429 | 456 => "The translation service's limit is used up for now. Try later.".into(),
            _ => format!(
                "The translation service said: {}",
                server_message(&r.body).unwrap_or_else(|| status_text(&r))
            ),
        },
    })
}

/// MyMemory's free service: no account needed.
pub(super) struct MyMemory<'a>(pub &'a dyn Transport);

impl ApiClient for MyMemory<'_> {
    fn base_url(&self) -> &'static str {
        "https://api.mymemory.translated.net"
    }
    fn transport(&self) -> &dyn Transport {
        self.0
    }
    fn error(&self, problem: Problem) -> ApiError {
        service_error(problem)
    }
}

impl MyMemory<'_> {
    pub(super) async fn translate(
        &self,
        text: &str,
        from: Option<&str>,
        to: &str,
    ) -> Result<Translation, ApiError> {
        let Some(from) = from else {
            return Err(ApiError::Other("This post doesn't say what language it's in, which MyMemory needs. Open it in the browser, or choose another service in Settings.".into()));
        };
        let pair = format!("{from}|{to}");
        let mut out = String::new();
        for part in chunks(text, MY_MEMORY_MAX) {
            let v: Value = self
                .get("/get", &[("q", part.as_str()), ("langpair", &pair)])
                .await?;
            let status = v["responseStatus"].as_u64().unwrap_or(200);
            let t = v["responseData"]["translatedText"]
                .as_str()
                .unwrap_or_default();
            if status != 200 || t.is_empty() {
                let detail = v["responseDetails"]
                    .as_str()
                    .unwrap_or("no translation came back");
                return Err(ApiError::Other(format!(
                    "MyMemory couldn't translate this: {detail}"
                )));
            }
            out.push_str(t);
        }
        Ok(Translation {
            text: out,
            from: Some(from.to_string()),
        })
    }
}

/// DeepL, with the person's own API key.
pub(super) struct DeepL<'a> {
    pub net: &'a dyn Transport,
    pub key: String,
}

impl ApiClient for DeepL<'_> {
    fn base_url(&self) -> &str {
        // Free accounts' keys end in ":fx" and use their own address.
        if self.key.ends_with(":fx") {
            "https://api-free.deepl.com"
        } else {
            "https://api.deepl.com"
        }
    }
    fn transport(&self) -> &dyn Transport {
        self.net
    }
    fn headers(&self) -> Vec<(String, String)> {
        vec![(
            "Authorization".into(),
            format!("DeepL-Auth-Key {}", self.key),
        )]
    }
    fn error(&self, problem: Problem) -> ApiError {
        service_error(problem)
    }
}

impl DeepL<'_> {
    pub(super) async fn translate(&self, text: &str, to: &str) -> Result<Translation, ApiError> {
        let body = json!({"text": [text], "target_lang": deepl_lang(to)});
        let v: Value = self.post("/v2/translate", &body).await?;
        let t = &v["translations"][0];
        Ok(Translation {
            text: t["text"]
                .as_str()
                .ok_or_else(|| ApiError::Other("DeepL sent back no translation.".into()))?
                .to_string(),
            from: t["detected_source_language"]
                .as_str()
                .map(str::to_lowercase),
        })
    }
}

/// DeepL wants upper case, and a variant for English and Portuguese.
fn deepl_lang(to: &str) -> String {
    match to {
        "en" => "EN-US".into(),
        "pt" => "PT-BR".into(),
        "zh" => "ZH-HANS".into(),
        other => other.to_uppercase(),
    }
}

/// A LibreTranslate server, such as one the person runs themselves. Its key goes in
/// the request body.
pub(super) struct Libre<'a> {
    pub net: &'a dyn Transport,
    pub server: &'a str,
    pub key: Option<String>,
}

impl ApiClient for Libre<'_> {
    fn base_url(&self) -> &str {
        self.server
    }
    fn transport(&self) -> &dyn Transport {
        self.net
    }
    fn error(&self, problem: Problem) -> ApiError {
        service_error(problem)
    }
}

impl Libre<'_> {
    pub(super) async fn translate(&self, text: &str, to: &str) -> Result<Translation, ApiError> {
        let mut body = json!({"q": text, "source": "auto", "target": to, "format": "text"});
        if let Some(k) = &self.key {
            body["api_key"] = k.as_str().into();
        }
        let v: Value = self.post("/translate", &body).await?;
        Ok(Translation {
            text: v["translatedText"]
                .as_str()
                .ok_or_else(|| ApiError::Other("LibreTranslate sent back no translation.".into()))?
                .to_string(),
            from: v["detectedLanguage"]["language"]
                .as_str()
                .map(str::to_lowercase),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::Method;
    use crate::net::fake::Fake;

    fn reply(text: &str) -> String {
        json!({"responseStatus": 200, "responseData": {"translatedText": text}}).to_string()
    }

    #[tokio::test]
    async fn my_memory_translates_in_pieces() {
        let net = Fake::default()
            .reply(200, &reply("A"))
            .reply(200, &reply("B"));
        let long = "Hello there. ".repeat(60);
        let t = MyMemory(&net)
            .translate(&long, Some("en"), "de")
            .await
            .unwrap();
        assert_eq!(t.text, "AB");
        let seen = net.seen();
        assert_eq!(seen.len(), 2);
        assert!(
            seen[0]
                .url
                .as_str()
                .starts_with("https://api.mymemory.translated.net/get?q=")
        );
        assert!(seen[0].url.as_str().ends_with("&langpair=en%7Cde"));
        let none = MyMemory(&net).translate("x", None, "de").await;
        assert!(none.is_err());
    }

    #[tokio::test]
    async fn deepl_signs_with_the_key() {
        let net = Fake::default().reply(
            200,
            r#"{"translations":[{"text":"Hallo","detected_source_language":"EN"}]}"#,
        );
        let deepl = DeepL {
            net: &net,
            key: "k:fx".into(),
        };
        let t = deepl.translate("Hello", "de").await.unwrap();
        assert_eq!(t.text, "Hallo");
        assert_eq!(t.from.as_deref(), Some("en"));
        let seen = &net.seen()[0];
        assert_eq!(seen.method, Method::Post);
        assert_eq!(seen.url.as_str(), "https://api-free.deepl.com/v2/translate");
        assert!(
            seen.headers
                .contains(&("Authorization".into(), "DeepL-Auth-Key k:fx".into()))
        );
        assert_eq!(deepl_lang("en"), "EN-US");
        assert_eq!(deepl_lang("ja"), "JA");
    }

    #[tokio::test]
    async fn libre_puts_the_key_in_the_body_and_words_errors() {
        let net = Fake::default()
            .reply(200, r#"{"translatedText":"Hola"}"#)
            .reply(403, "{}")
            .reply(429, "{}")
            .reply(400, r#"{"error":"bad language"}"#);
        let libre = Libre {
            net: &net,
            server: "https://lt.example",
            key: Some("s".into()),
        };
        assert_eq!(libre.translate("Hi", "es").await.unwrap().text, "Hola");
        let body: Value = serde_json::from_slice(net.seen()[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(body["api_key"], "s");
        assert_eq!(net.seen()[0].url.as_str(), "https://lt.example/translate");
        let mut errors = Vec::new();
        for _ in 0..3 {
            errors.push(libre.translate("Hi", "es").await.unwrap_err().to_string());
        }
        assert_eq!(
            errors[0],
            "The translation service turned down the API key."
        );
        assert_eq!(
            errors[1],
            "The translation service's limit is used up for now. Try later."
        );
        assert_eq!(errors[2], "The translation service said: bad language");
    }
}
