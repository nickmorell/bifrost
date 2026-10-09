//! Reading public records straight from the server that keeps them (a person's PDS),
//! for record types Bluesky's app view doesn't serve, such as articles.

use super::super::ApiError;
use super::super::client::ApiClient;
use super::super::web::Web;
use crate::net::Transport;
use serde::Deserialize;
use serde_json::Value;

/// The directory that knows where each `did:plc` account lives.
struct Plc<'a>(&'a dyn Transport);

impl ApiClient for Plc<'_> {
    fn base_url(&self) -> &'static str {
        "https://plc.directory"
    }

    fn transport(&self) -> &dyn Transport {
        self.0
    }
}

/// One account's PDS.
pub struct Pds<'a> {
    base: String,
    net: &'a dyn Transport,
}

impl ApiClient for Pds<'_> {
    fn base_url(&self) -> &str {
        &self.base
    }

    fn transport(&self) -> &dyn Transport {
        self.net
    }
}

/// A record and where it is.
#[derive(Debug, Clone, Deserialize)]
pub struct Record {
    pub uri: String,
    pub value: Value,
}

#[derive(Deserialize)]
struct Records {
    #[serde(default)]
    records: Vec<Value>,
}

impl<'a> Pds<'a> {
    /// Finds where `did` keeps its records.
    pub async fn of(net: &'a dyn Transport, did: &str) -> Result<Self, ApiError> {
        #[derive(Deserialize)]
        struct Service {
            id: String,
            #[serde(rename = "serviceEndpoint")]
            endpoint: Value,
        }
        #[derive(Deserialize)]
        struct Doc {
            #[serde(default)]
            service: Vec<Service>,
        }
        let doc: Doc = if did.starts_with("did:plc:") && plain(did) {
            Plc(net).get(&format!("/{did}"), &[]).await?
        } else if let Some(host) = did.strip_prefix("did:web:").filter(|h| plain(h)) {
            Web(net)
                .get(&format!("https://{host}/.well-known/did.json"), &[])
                .await?
        } else {
            return Err(ApiError::Other(format!("can't look up {did}")));
        };
        let base = doc
            .service
            .iter()
            .find(|s| s.id == "#atproto_pds" || s.id.ends_with("#atproto_pds"))
            .and_then(|s| s.endpoint.as_str())
            .filter(|e| e.starts_with("https://"))
            .map(|e| e.trim_end_matches('/').to_string())
            .ok_or_else(|| ApiError::Other(format!("{did} has no server")))?;
        Ok(Self { base, net })
    }

    #[cfg(test)]
    pub fn at(base: &str, net: &'a dyn Transport) -> Self {
        Self {
            base: base.into(),
            net,
        }
    }

    pub async fn record(
        &self,
        did: &str,
        collection: &str,
        rkey: &str,
    ) -> Result<Record, ApiError> {
        self.get(
            "/xrpc/com.atproto.repo.getRecord",
            &[("repo", did), ("collection", collection), ("rkey", rkey)],
        )
        .await
    }

    /// Up to `limit` of `did`'s records in `collection`, newest first. Ones that
    /// don't read are skipped.
    pub async fn records(
        &self,
        did: &str,
        collection: &str,
        limit: u32,
    ) -> Result<Vec<Record>, ApiError> {
        let limit = limit.to_string();
        let out: Records = self
            .get(
                "/xrpc/com.atproto.repo.listRecords",
                &[("repo", did), ("collection", collection), ("limit", &limit)],
            )
            .await?;
        Ok(super::super::lenient_items(out.records))
    }

    /// Where a picture (blob) kept on this server can be downloaded.
    pub fn blob_url(&self, did: &str, blob: &Value) -> Option<String> {
        let cid = blob["ref"]["$link"].as_str().filter(|c| plain(c))?;
        Some(format!(
            "{}/xrpc/com.atproto.sync.getBlob?did={did}&cid={cid}",
            self.base
        ))
    }
}

/// Letters, digits and the few marks DIDs, hosts and CIDs use: nothing that could
/// change where a request goes.
fn plain(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_:".contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fake::Fake;

    #[tokio::test]
    async fn finds_the_server_from_the_directory() {
        let net = Fake::default()
            .reply(
                200,
                r##"{"service":[{"id":"#atproto_pds","type":"AtprotoPersonalDataServer","serviceEndpoint":"https://pds.example/"}]}"##,
            )
            .reply(200, r#"{"service":[]}"#)
            .reply(
                200,
                r##"{"service":[{"id":"#atproto_pds","serviceEndpoint":"http://insecure.example"}]}"##,
            );
        let pds = Pds::of(&net, "did:plc:abc").await.unwrap();
        assert_eq!(pds.base_url(), "https://pds.example");
        assert_eq!(
            net.seen()[0].url.as_str(),
            "https://plc.directory/did:plc:abc"
        );
        assert!(Pds::of(&net, "did:web:blog.example").await.is_err());
        assert_eq!(
            net.seen()[1].url.as_str(),
            "https://blog.example/.well-known/did.json"
        );
        assert!(Pds::of(&net, "did:plc:x").await.is_err(), "http only");
        assert!(Pds::of(&net, "did:plc:a/../b").await.is_err());
        assert_eq!(net.seen().len(), 3);
    }

    #[tokio::test]
    async fn reads_records_and_blob_addresses() {
        let net = Fake::default()
            .reply(
                200,
                r#"{"uri":"at://d/c/k","cid":"v","value":{"title":"T"}}"#,
            )
            .reply(
                200,
                r#"{"records":[{"uri":"at://d/c/1","cid":"a","value":{}},{"nope":1}]}"#,
            );
        let pds = Pds::at("https://pds.example", &net);
        let r = pds.record("did:plc:d", "c.x", "k").await.unwrap();
        assert_eq!(r.value["title"], "T");
        assert_eq!(
            net.seen()[0].url.as_str(),
            "https://pds.example/xrpc/com.atproto.repo.getRecord?repo=did%3Aplc%3Ad&collection=c.x&rkey=k"
        );
        assert_eq!(pds.records("did:plc:d", "c.x", 5).await.unwrap().len(), 1);
        let blob = serde_json::json!({"ref": {"$link": "bafy1"}});
        assert_eq!(
            pds.blob_url("did:plc:d", &blob).as_deref(),
            Some("https://pds.example/xrpc/com.atproto.sync.getBlob?did=did:plc:d&cid=bafy1")
        );
        let bad = serde_json::json!({"ref": {"$link": "a&b"}});
        assert_eq!(pds.blob_url("did:plc:d", &bad), None);
    }
}
