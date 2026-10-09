//! The flow every plain-HTTP client shares. A client says where it talks to, how it
//! signs in and how to word its errors; [`ApiClient`] does the rest: building the URL,
//! adding headers, capping the response size, checking the status and decoding JSON.
//!
//! Bluesky and chat calls go through atrium's XRPC client instead (see `auth::xrpc`),
//! which sends over the same [`Transport`].

use super::ApiError;
use crate::net::{Failure, MAX_RESPONSE, Method, Request, Response, Transport};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::future::Future;

/// Something that went wrong, before the client words it as an [`ApiError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    Unreachable(String),
    TooBig,
    /// The server answered with a status outside 200-299.
    Status(Response),
}

impl From<Failure> for Problem {
    fn from(f: Failure) -> Self {
        match f {
            Failure::Unreachable(e) => Problem::Unreachable(e),
            Failure::TooBig => Problem::TooBig,
        }
    }
}

pub trait ApiClient: Sync {
    /// Prepended to every path. Empty for a client that is given whole URLs.
    fn base_url(&self) -> &str;

    fn transport(&self) -> &dyn Transport;

    /// Headers every request carries, such as an API key.
    fn headers(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    fn max_response(&self) -> u64 {
        MAX_RESPONSE
    }

    /// Words a problem for the person using the app.
    fn error(&self, problem: Problem) -> ApiError {
        ApiError::Other(match problem {
            Problem::Unreachable(e) => e,
            Problem::TooBig => "the file is too large".into(),
            Problem::Status(r) => status_text(&r),
        })
    }

    /// A request to `path` with the client's headers and size cap.
    fn request(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Request, ApiError> {
        let address = format!("{}{path}", self.base_url());
        let url = if query.is_empty() {
            url::Url::parse(&address)
        } else {
            url::Url::parse_with_params(&address, query)
        }
        .map_err(|e| ApiError::Other(format!("not a web address: {e}")))?;
        Ok(Request {
            method,
            url,
            headers: self.headers(),
            body: None,
            timeout: None,
            limit: self.max_response(),
        })
    }

    /// Sends a request and hands back whatever status came back.
    fn send(&self, req: Request) -> impl Future<Output = Result<Response, ApiError>> + Send {
        async move {
            self.transport()
                .send(req)
                .await
                .map_err(|f| self.error(f.into()))
        }
    }

    /// Sends a request and hands back the body of a successful reply.
    fn fetch(&self, req: Request) -> impl Future<Output = Result<Vec<u8>, ApiError>> + Send {
        async move {
            let resp = self.send(req).await?;
            if resp.is_success() {
                Ok(resp.body)
            } else {
                Err(self.error(Problem::Status(resp)))
            }
        }
    }

    /// Sends a request and decodes a successful reply's JSON.
    fn json<T: DeserializeOwned + Send>(
        &self,
        req: Request,
    ) -> impl Future<Output = Result<T, ApiError>> + Send {
        async move {
            let body = self.fetch(req).await?;
            serde_json::from_slice(&body).map_err(|e| ApiError::Decode(e.to_string()))
        }
    }

    fn get<T: DeserializeOwned + Send>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> impl Future<Output = Result<T, ApiError>> + Send {
        let req = self.request(Method::Get, path, query);
        async move { self.json(req?).await }
    }

    fn post<B: Serialize, T: DeserializeOwned + Send>(
        &self,
        path: &str,
        body: &B,
    ) -> impl Future<Output = Result<T, ApiError>> + Send {
        let req = serde_json::to_vec(body)
            .map_err(|e| ApiError::Decode(e.to_string()))
            .and_then(|b| {
                Ok(self
                    .request(Method::Post, path, &[])?
                    .body("application/json", b))
            });
        async move { self.json(req?).await }
    }
}

/// The `message` or `error` a JSON error reply carries, if any.
pub fn server_message(body: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(body).ok()?;
    v.get("message")
        .or_else(|| v.get("error"))
        .and_then(Value::as_str)
        .filter(|m| !m.is_empty())
        .map(str::to_string)
}

/// "the server answered 404 Not Found", with the server's own words when it gave some.
pub fn status_text(resp: &Response) -> String {
    let reason = reqwest::StatusCode::from_u16(resp.status)
        .ok()
        .and_then(|s| s.canonical_reason())
        .unwrap_or_default();
    match server_message(&resp.body) {
        Some(m) => format!("the server answered {} {reason}: {m}", resp.status),
        None => format!("the server answered {} {reason}", resp.status),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fake::Fake;

    struct Example(Fake);

    impl ApiClient for Example {
        fn base_url(&self) -> &'static str {
            "https://example.com/api"
        }
        fn transport(&self) -> &dyn Transport {
            &self.0
        }
        fn headers(&self) -> Vec<(String, String)> {
            vec![("Authorization".into(), "Key k".into())]
        }
        fn max_response(&self) -> u64 {
            32
        }
    }

    #[tokio::test]
    async fn builds_requests_and_decodes_json() {
        let c = Example(Fake::default().reply(200, r#"{"n":1}"#));
        let v: Value = c.get("/x", &[("q", "a b")]).await.unwrap();
        assert_eq!(v["n"], 1);
        let seen = c.0.seen();
        assert_eq!(seen[0].url.as_str(), "https://example.com/api/x?q=a+b");
        assert_eq!(seen[0].method, Method::Get);
        assert_eq!(
            seen[0].headers,
            vec![("Authorization".into(), "Key k".into())]
        );
        assert_eq!(seen[0].limit, 32);
    }

    #[tokio::test]
    async fn posts_json() {
        let c = Example(Fake::default().reply(200, "{}"));
        let _: Value = c.post("/t", &serde_json::json!({"a": 1})).await.unwrap();
        let seen = c.0.seen();
        assert_eq!(seen[0].method, Method::Post);
        assert_eq!(seen[0].body.as_deref(), Some(&br#"{"a":1}"#[..]));
        assert!(
            seen[0]
                .headers
                .contains(&("Content-Type".into(), "application/json".into()))
        );
    }

    #[tokio::test]
    async fn words_failures() {
        let c = Example(
            Fake::default()
                .reply(404, r#"{"message":"no such thing"}"#)
                .reply(500, "oops")
                .reply(200, "this reply is more than thirty-two bytes long")
                .fail(Failure::Unreachable("offline".into()))
                .reply(200, "not json"),
        );
        let mut errors = Vec::new();
        for _ in 0..5 {
            errors.push(c.get::<Value>("/x", &[]).await.unwrap_err().to_string());
        }
        assert_eq!(
            errors[0],
            "the server answered 404 Not Found: no such thing"
        );
        assert_eq!(errors[1], "the server answered 500 Internal Server Error");
        assert_eq!(errors[2], "the file is too large");
        assert_eq!(errors[3], "offline");
        assert!(errors[4].starts_with("could not read"));
    }
}
