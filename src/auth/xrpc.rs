//! The HTTP client atrium sends Bluesky and chat calls through. It hands every request
//! to the shared [`Transport`], so XRPC calls use the same connection pool, timeouts and
//! response size cap as every other request BiFrost makes.

use crate::net::{self, MAX_RESPONSE, Method, Request, Transport};
use atrium_api::xrpc::http::{self, Response};
use atrium_api::xrpc::{HttpClient, XrpcClient};
use std::error::Error;
use std::time::Duration;

/// Long enough to upload a few megabytes of images on a slow line.
const TIMEOUT: Duration = Duration::from_mins(1);

/// An XRPC client for one server.
#[derive(Clone)]
pub struct Xrpc {
    base: String,
    net: &'static dyn Transport,
}

impl Xrpc {
    /// A client for `base` over the shared transport.
    pub fn new(base: &str) -> Self {
        Self::over(base, net::shared())
    }

    fn over(base: &str, net: &'static dyn Transport) -> Self {
        Self {
            base: base.to_string(),
            net,
        }
    }
}

type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// atrium's request as a plain transport request.
fn to_request(request: http::Request<Vec<u8>>) -> Result<Request, BoxError> {
    let (parts, body) = request.into_parts();
    let method = match parts.method {
        http::Method::GET => Method::Get,
        http::Method::POST => Method::Post,
        other => return Err(format!("unsupported method {other}").into()),
    };
    let headers = parts
        .headers
        .iter()
        .filter_map(|(k, v)| Some((k.to_string(), v.to_str().ok()?.to_string())))
        .collect();
    Ok(Request {
        url: url::Url::parse(&parts.uri.to_string())?,
        body: (method == Method::Post).then_some(body),
        method,
        headers,
        timeout: Some(TIMEOUT),
        limit: MAX_RESPONSE,
    })
}

impl HttpClient for Xrpc {
    async fn send_http(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, BoxError> {
        let reply = self.net.send(to_request(request)?).await?;
        let mut builder = Response::builder().status(reply.status);
        for (name, value) in reply.headers {
            builder = builder.header(name, value);
        }
        Ok(builder.body(reply.body)?)
    }
}

impl XrpcClient for Xrpc {
    fn base_uri(&self) -> String {
        self.base.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::Failure;
    use crate::net::fake::Fake;
    use atrium_api::agent::atp_agent::AtpAgent;
    use atrium_api::agent::atp_agent::store::MemorySessionStore;
    use atrium_api::types::string::Handle;

    fn leak(fake: Fake) -> &'static Fake {
        Box::leak(Box::new(fake))
    }

    #[tokio::test]
    async fn calls_go_through_the_transport_with_its_limits() {
        let net = leak(Fake::default().reply_with(
            200,
            &[("content-type", "application/json; charset=utf-8")],
            r#"{"did":"did:plc:a"}"#,
        ));
        let agent = AtpAgent::new(
            Xrpc::over("https://pds.test", net),
            MemorySessionStore::default(),
        );
        let handle = Handle::new("ann.test".into()).unwrap();
        let out = agent
            .api
            .com
            .atproto
            .identity
            .resolve_handle(
                atrium_api::com::atproto::identity::resolve_handle::ParametersData { handle }
                    .into(),
            )
            .await
            .unwrap();
        assert_eq!(out.did.as_str(), "did:plc:a");
        let seen = net.seen();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].method, Method::Get);
        assert_eq!(
            seen[0].url.as_str(),
            "https://pds.test/xrpc/com.atproto.identity.resolveHandle?handle=ann.test"
        );
        assert_eq!(seen[0].limit, MAX_RESPONSE);
        assert_eq!(seen[0].timeout, Some(TIMEOUT));
        assert_eq!(seen[0].body, None);
    }

    #[tokio::test]
    async fn posts_carry_their_body_and_headers() {
        let net = leak(Fake::default().reply(200, ""));
        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri("https://pds.test/xrpc/x.y.z")
            .header("Content-Type", "application/json")
            .body(b"{}".to_vec())
            .unwrap();
        let reply = Xrpc::over("https://pds.test", net)
            .send_http(request)
            .await
            .unwrap();
        assert_eq!(reply.status(), 200);
        let seen = net.seen();
        assert_eq!(seen[0].method, Method::Post);
        assert_eq!(seen[0].body.as_deref(), Some(&b"{}"[..]));
        assert!(
            seen[0]
                .headers
                .contains(&("content-type".into(), "application/json".into()))
        );
    }

    #[tokio::test]
    async fn an_oversized_reply_is_an_error() {
        let net = leak(Fake::default().fail(Failure::TooBig));
        let request = http::Request::builder()
            .uri("https://pds.test/xrpc/x.y.z")
            .body(Vec::new())
            .unwrap();
        let err = Xrpc::over("https://pds.test", net)
            .send_http(request)
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "the reply was too large");
    }

    #[tokio::test]
    async fn the_server_s_headers_reach_atrium() {
        let net = leak(Fake::default().reply_with(
            401,
            &[("www-authenticate", "DPoP error=\"use_dpop_nonce\"")],
            "",
        ));
        let request = http::Request::builder()
            .uri("https://pds.test/xrpc/x.y.z")
            .body(Vec::new())
            .unwrap();
        let reply = Xrpc::over("https://pds.test", net)
            .send_http(request)
            .await
            .unwrap();
        assert_eq!(reply.status(), 401);
        assert_eq!(
            reply.headers()["www-authenticate"],
            "DPoP error=\"use_dpop_nonce\""
        );
    }
}
