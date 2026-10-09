//! Moving bytes over HTTP. A [`Transport`] takes a plain [`Request`] and gives back
//! the status, headers and body, never more than the request's size cap. The real one
//! uses reqwest; tests use [`fake::Fake`], so no test touches the network.
//!
//! Every request BiFrost makes goes through here: Bluesky and chat calls (atrium's
//! XRPC client in `auth::xrpc`), link cards, images, translation and video.

pub mod socket;

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;
use std::time::Duration;

/// The largest response a request reads unless it says otherwise.
pub const MAX_RESPONSE: u64 = 25 * 1024 * 1024;

/// A future a transport hands back. Boxed so `dyn Transport` works.
pub type Pending<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

/// One HTTP request, as plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub method: Method,
    pub url: url::Url,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    /// How long the whole request may take, when not the transport's usual limit.
    pub timeout: Option<Duration>,
    /// The largest body to read back.
    pub limit: u64,
}

impl Request {
    #[must_use]
    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_string(), value.into()));
        self
    }

    #[must_use]
    pub fn body(mut self, content_type: &str, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self.header("Content-Type", content_type)
    }

    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    #[must_use]
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = limit;
        self
    }
}

/// What came back: any status, with its headers and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Where the reply came from, when the server redirected the request elsewhere.
    pub moved_to: Option<url::Url>,
}

impl Response {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Why no response came back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The server couldn't be reached, or the connection dropped.
    Unreachable(String),
    /// The body was larger than the request's limit.
    TooBig,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Unreachable(e) => f.write_str(e),
            Failure::TooBig => f.write_str("the reply was too large"),
        }
    }
}

impl std::error::Error for Failure {}

pub trait Transport: Send + Sync {
    fn send(&self, req: Request) -> Pending<'_, Result<Response, Failure>>;
}

/// The real transport: one pooled reqwest client.
pub struct Http(reqwest::Client);

/// The transport everything shares, so all requests use one connection pool. Only for
/// the app's own runtime: a pooled connection belongs to the runtime that opened it, so
/// the video player, which runs on its own, keeps its own [`Http`].
pub fn shared() -> &'static Http {
    static NET: OnceLock<Http> = OnceLock::new();
    NET.get_or_init(Http::default)
}

impl Default for Http {
    fn default() -> Self {
        Self(
            reqwest::Client::builder()
                .user_agent(concat!("BiFrost/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        )
    }
}

impl Transport for Http {
    fn send(&self, req: Request) -> Pending<'_, Result<Response, Failure>> {
        Box::pin(async move {
            let asked = req.url.clone();
            let mut builder = match req.method {
                Method::Get => self.0.get(req.url),
                Method::Post => self.0.post(req.url),
            };
            for (name, value) in req.headers {
                builder = builder.header(name, value);
            }
            if let Some(body) = req.body {
                builder = builder.body(body);
            }
            if let Some(t) = req.timeout {
                builder = builder.timeout(t);
            }
            let resp = builder
                .send()
                .await
                .map_err(|e| Failure::Unreachable(e.to_string()))?;
            let moved_to = (resp.url() != &asked).then(|| resp.url().clone());
            let status = resp.status().as_u16();
            let headers = resp
                .headers()
                .iter()
                .filter_map(|(k, v)| Some((k.to_string(), v.to_str().ok()?.to_string())))
                .collect();
            let body = read_capped(resp, req.limit).await?;
            Ok(Response {
                status,
                headers,
                body,
                moved_to,
            })
        })
    }
}

/// Reads a response body in pieces, stopping once it passes `cap` bytes: a server can
/// leave out Content-Length and stream without end.
async fn read_capped(mut resp: reqwest::Response, cap: u64) -> Result<Vec<u8>, Failure> {
    if resp.content_length().is_some_and(|n| n > cap) {
        return Err(Failure::TooBig);
    }
    let mut out = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| Failure::Unreachable(e.to_string()))?
    {
        if (out.len() + chunk.len()) as u64 > cap {
            return Err(Failure::TooBig);
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod fake {
    use super::{Failure, Pending, Request, Response, Transport};
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// Answers requests from a queue and remembers what was asked.
    #[derive(Default)]
    pub(crate) struct Fake {
        replies: Mutex<VecDeque<Result<Response, Failure>>>,
        seen: Mutex<Vec<Request>>,
    }

    impl Fake {
        /// Queues a reply with this status and body.
        pub(crate) fn reply(self, status: u16, body: &str) -> Self {
            self.reply_with(status, &[], body)
        }

        /// Queues a reply with this status, these headers and this body.
        pub(crate) fn reply_with(self, status: u16, headers: &[(&str, &str)], body: &str) -> Self {
            self.push(Ok(Response {
                status,
                headers: headers
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
                body: body.as_bytes().to_vec(),
                moved_to: None,
            }))
        }

        /// Queues a reply that came from `to` after a redirect.
        pub(crate) fn reply_moved(self, to: &str, status: u16, body: &str) -> Self {
            self.push(Ok(Response {
                status,
                headers: Vec::new(),
                body: body.as_bytes().to_vec(),
                moved_to: url::Url::parse(to).ok(),
            }))
        }

        pub(crate) fn fail(self, failure: Failure) -> Self {
            self.push(Err(failure))
        }

        fn push(self, reply: Result<Response, Failure>) -> Self {
            if let Ok(mut q) = self.replies.lock() {
                q.push_back(reply);
            }
            self
        }

        /// The requests sent so far.
        pub(crate) fn seen(&self) -> Vec<Request> {
            self.seen.lock().map(|s| s.clone()).unwrap_or_default()
        }
    }

    impl Transport for Fake {
        fn send(&self, req: Request) -> Pending<'_, Result<Response, Failure>> {
            let limit = req.limit;
            if let Ok(mut s) = self.seen.lock() {
                s.push(req);
            }
            let reply = self
                .replies
                .lock()
                .ok()
                .and_then(|mut q| q.pop_front())
                .unwrap_or_else(|| Err(Failure::Unreachable("no reply queued".into())));
            Box::pin(async move {
                match reply {
                    Ok(r) if r.body.len() as u64 > limit => Err(Failure::TooBig),
                    other => other,
                }
            })
        }
    }
}
