//! Streamplace (stream.place), the live streaming service on the AT Protocol: who is
//! live, a streamer's latest stream and past ones, where to play them, and the chat,
//! which the node pushes over a websocket. Sending chat goes through your own Bluesky
//! account instead (`Client::send_live_chat`).

use super::ApiError;
use super::client::{ApiClient, Problem, status_text};
use crate::model::{
    AtUri, Author, Did, Facet, LiveChat, LiveEvent, LiveStream, Page, PastStream, PostRecord,
};
use crate::net::socket;
use crate::net::{Method, Transport};
use futures_util::Stream;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

/// The Streamplace node BiFrost talks to.
pub const NODE: &str = "https://stream.place";
/// The record type of a chat message.
pub const CHAT_MESSAGE: &str = "place.stream.chat.message";

/// How long to wait before connecting to the chat again, at most.
const MAX_RETRY: Duration = Duration::from_secs(30);

/// The node's public calls.
pub struct Node<'a>(pub &'a dyn Transport);

impl ApiClient for Node<'_> {
    fn base_url(&self) -> &'static str {
        NODE
    }

    fn transport(&self) -> &dyn Transport {
        self.0
    }

    fn error(&self, problem: Problem) -> ApiError {
        ApiError::Other(match problem {
            Problem::Unreachable(e) => format!("Couldn't reach Streamplace: {e}"),
            Problem::TooBig => "Streamplace sent more than expected".into(),
            Problem::Status(r) => format!("Streamplace answered: {}", status_text(&r)),
        })
    }
}

impl Node<'_> {
    /// The streams that are live now, the most watched first.
    pub async fn live_now(&self) -> Result<Vec<LiveStream>, ApiError> {
        #[derive(Deserialize)]
        struct Out {
            #[serde(default)]
            streams: Vec<Value>,
        }
        let out: Out = self
            .get("/xrpc/place.stream.live.getLiveUsers", &[("limit", "50")])
            .await?;
        Ok(out.streams.iter().filter_map(stream).collect())
    }

    /// `actor`'s latest stream, which may have ended, or `None` if they never streamed.
    pub async fn latest(&self, actor: &str) -> Result<Option<LiveStream>, ApiError> {
        let req = self.request(Method::Get, &format!("/api/livestream/{actor}"), &[])?;
        let resp = self.send(req).await?;
        match resp.status {
            404 => Ok(None),
            s if (200..300).contains(&s) => {
                let v: Value = serde_json::from_slice(&resp.body)
                    .map_err(|e| ApiError::Decode(e.to_string()))?;
                Ok(stream(&v))
            }
            _ => Err(self.error(Problem::Status(resp))),
        }
    }

    /// A page of `actor`'s past streams, newest first.
    pub async fn past(
        &self,
        actor: &str,
        cursor: Option<&str>,
    ) -> Result<Page<PastStream>, ApiError> {
        #[derive(Deserialize)]
        struct Out {
            #[serde(default)]
            videos: Vec<Value>,
            cursor: Option<String>,
        }
        let mut query = vec![("repo", actor), ("limit", "25")];
        query.extend(cursor.map(|c| ("cursor", c)));
        let out: Out = self
            .get("/xrpc/place.stream.media.getVideoList", &query)
            .await?;
        Ok(Page {
            items: out.videos.iter().filter_map(past_stream).collect(),
            cursor: out.cursor,
        })
    }
}

/// Where the live stream of `streamer` plays, as HLS.
pub fn live_playlist(streamer: &Did) -> String {
    url::Url::parse_with_params(
        &format!("{NODE}/xrpc/place.stream.playback.getLivePlaylist"),
        &[("streamer", streamer.as_str())],
    )
    .map_or_else(|_| String::new(), String::from)
}

/// Where a past stream plays, as HLS.
pub fn past_playlist(video: &AtUri) -> String {
    url::Url::parse_with_params(
        &format!("{NODE}/xrpc/place.stream.playback.getVideoPlaylist"),
        &[("uri", video.as_str()), ("ext", "hls.m3u8")],
    )
    .map_or_else(|_| String::new(), String::from)
}

/// A streamer's page on stream.place, for opening in the browser.
pub fn web_page(actor: &str) -> String {
    format!("{NODE}/{actor}")
}

/// The streamer named by a stream.place link (`https://stream.place/<handle or DID>`).
pub fn streamer_in(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    let host = u.host_str()?.trim_start_matches("www.");
    if host != "stream.place" {
        return None;
    }
    let mut parts = u.path_segments()?.filter(|p| !p.is_empty());
    let actor = parts.next()?;
    (parts.next().is_none() && (actor.contains('.') || actor.starts_with("did:")))
        .then(|| actor.to_string())
}

/// The chat and other happenings of `streamer`'s stream, for as long as the stream is
/// read: it reconnects when the connection drops, waiting longer each time.
pub fn events(streamer: Did) -> impl Stream<Item = LiveEvent> + Send {
    let url = format!("wss://stream.place/api/websocket/{streamer}");
    futures_util::stream::unfold(
        (url, None::<socket::Socket>, 0u32),
        |(url, mut open, mut tries)| async move {
            loop {
                match open.as_mut() {
                    None => {
                        if tries > 0 {
                            let wait = Duration::from_secs(2u64.saturating_pow(tries.min(5)));
                            tokio::time::sleep(wait.min(MAX_RETRY)).await;
                        }
                        match socket::connect(&url).await {
                            Ok(s) => {
                                tries = 0;
                                return Some((LiveEvent::Connected(true), (url, Some(s), tries)));
                            }
                            Err(_) => tries += 1,
                        }
                    }
                    Some(s) => {
                        let Some(Ok(text)) = s.next_text().await else {
                            tries = 1;
                            return Some((LiveEvent::Connected(false), (url, None, tries)));
                        };
                        if let Some(event) = event(&text) {
                            return Some((event, (url, open, tries)));
                        }
                    }
                }
            }
        },
    )
}

/// One message from the node's websocket, if it is one BiFrost uses.
pub fn event(text: &str) -> Option<LiveEvent> {
    let v: Value = serde_json::from_str(text).ok()?;
    match v["$type"].as_str()? {
        "place.stream.chat.defs#messageView" if v["deleted"] == true => {
            Some(LiveEvent::Removed(v["uri"].as_str()?.into()))
        }
        "place.stream.chat.defs#messageView" => chat(&v).map(LiveEvent::Chat),
        "place.stream.chat.gate" => Some(LiveEvent::Removed(v["hiddenMessage"].as_str()?.into())),
        "place.stream.defs#blockView" => {
            Some(LiveEvent::Blocked(v["record"]["subject"].as_str()?.into()))
        }
        "place.stream.livestream#viewerCount" => Some(LiveEvent::Viewers(v["count"].as_u64()?)),
        "place.stream.livestream#livestreamView" => stream(&v).map(LiveEvent::Stream),
        "place.stream.chat.defs#pinnedRecordView" => Some(LiveEvent::Pinned(chat(&v["message"]))),
        "place.stream.chat.pinnedRecord" if v["deleted"] == true => Some(LiveEvent::Pinned(None)),
        _ => None,
    }
}

/// A chat message from its `messageView`.
fn chat(v: &Value) -> Option<LiveChat> {
    let author: Author = serde_json::from_value(v["author"].clone()).ok()?;
    let record = &v["record"];
    let color = &v["chatProfile"]["color"];
    let channel = |c: &str| color[c].as_u64().and_then(|n| u8::try_from(n).ok());
    let color = color
        .is_object()
        .then(|| Some([channel("red")?, channel("green")?, channel("blue")?]))
        .flatten();
    let reply_to = v.get("replyTo").and_then(|r| {
        let who: Author = serde_json::from_value(r["author"].clone()).ok()?;
        Some((
            who.name().to_string(),
            r["record"]["text"].as_str()?.to_string(),
        ))
    });
    Some(LiveChat {
        uri: v["uri"].as_str()?.into(),
        body: PostRecord {
            text: record["text"].as_str().unwrap_or_default().to_string(),
            facets: serde_json::from_value::<Vec<Facet>>(record["facets"].clone())
                .unwrap_or_default(),
            created_at: record["createdAt"].as_str().unwrap_or_default().to_string(),
            ..PostRecord::default()
        },
        color,
        author,
        reply_to,
    })
}

/// A stream from its `livestreamView`.
fn stream(v: &Value) -> Option<LiveStream> {
    let record = &v["record"];
    let did: Did = v["author"]["did"].as_str()?.into();
    let thumb = record["thumb"]["ref"]["$link"]
        .as_str()
        .map(|cid| format!("https://cdn.bsky.app/img/feed_thumbnail/plain/{did}/{cid}@jpeg"));
    let activity = &record["activity"];
    let activity = activity["name"]
        .as_str()
        .map(str::to_string)
        .or_else(|| activity["label"].as_str().map(label));
    Some(LiveStream {
        uri: v["uri"].as_str()?.into(),
        handle: v["author"]["handle"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        title: record["title"].as_str().unwrap_or_default().to_string(),
        started: record["createdAt"].as_str().unwrap_or_default().to_string(),
        ended: record["endedAt"].as_str().map(str::to_string),
        viewers: v["viewerCount"]["count"].as_u64(),
        thumb,
        activity,
        tags: record["tags"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| t.as_str())
            .filter(|t| !t.starts_with("lang:"))
            .map(str::to_string)
            .collect(),
        did,
    })
}

/// A past stream from its `videoView`.
fn past_stream(v: &Value) -> Option<PastStream> {
    let record = &v["record"];
    let ms = v["viewCounts"]["durationMs"]
        .as_u64()
        .or_else(|| record["duration"].as_u64().map(|ns| ns / 1_000_000));
    Some(PastStream {
        uri: v["uri"].as_str()?.into(),
        title: record["title"].as_str().unwrap_or_default().to_string(),
        created: record["createdAt"].as_str().unwrap_or_default().to_string(),
        seconds: ms.map(|ms| ms / 1000),
    })
}

/// Words for a stream category such as `software_dev`: "Software dev".
fn label(id: &str) -> String {
    let words = id.replace('_', " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
