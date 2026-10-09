//! The Bluesky side of live streams: the profiles of people streaming, and writing
//! chat messages, which are records in your own repository like posts.

use super::{AnyAgent, streamplace};
use super::{ApiError, Client, conv, now_iso, xrpc_err};
use crate::model::{AtUri, Author, Did};
use serde_json::{Value, json};

/// Profiles the server returns per call.
const PROFILES_PER_CALL: usize = 25;

impl Client {
    /// Profiles for `dids`, in no particular order; ones that can't be found are left
    /// out.
    pub async fn profiles(&self, dids: &[Did]) -> Result<Vec<Author>, ApiError> {
        let mut out = Vec::new();
        for chunk in dids.chunks(PROFILES_PER_CALL) {
            let actors: Vec<&str> = chunk.iter().map(Did::as_str).collect();
            let got = xrpc!(self, |a| a
                .api
                .app
                .bsky
                .actor
                .get_profiles(conv(json!({ "actors": actors }))?));
            let v: Value = conv(&*got)?;
            out.extend(
                v["profiles"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| serde_json::from_value(p.clone()).ok()),
            );
        }
        Ok(out)
    }

    /// Sends `text` to `streamer`'s chat. Links and mentions become facets, as in
    /// posts; hashtags stay plain text, since stream chat doesn't take them.
    pub async fn send_live_chat(&self, streamer: &Did, text: &str) -> Result<AtUri, ApiError> {
        let facets: Vec<Value> = self
            .facets(text)
            .await
            .into_iter()
            .filter(|f| f["features"][0]["$type"] != "app.bsky.richtext.facet#tag")
            .collect();
        let mut record = json!({
            "$type": streamplace::CHAT_MESSAGE,
            "text": text,
            "createdAt": now_iso(),
            "streamer": streamer,
        });
        if !facets.is_empty() {
            record["facets"] = Value::Array(facets);
        }
        self.create_record(streamplace::CHAT_MESSAGE, record).await
    }
}
