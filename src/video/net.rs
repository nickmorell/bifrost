//! Downloading a video's playlists and segments.

use crate::api::ApiError;
use crate::api::client::{ApiClient, Problem, status_text};
use crate::net::{Http, Method, Transport};
use std::sync::OnceLock;

/// The most a playlist or a segment may be. Bluesky's segments are a few hundred KB.
const MAX_PLAYLIST: u64 = 1024 * 1024;
const MAX_SEGMENT: u64 = 16 * 1024 * 1024;

/// Wherever a video's playlists and segments are; they come as whole addresses.
struct Hls<'a>(&'a dyn Transport);

impl ApiClient for Hls<'_> {
    fn base_url(&self) -> &'static str {
        ""
    }

    fn transport(&self) -> &dyn Transport {
        self.0
    }

    fn error(&self, problem: Problem) -> ApiError {
        ApiError::Other(match problem {
            Problem::Unreachable(e) => e,
            Problem::TooBig => "The video is larger than expected".into(),
            Problem::Status(r) => status_text(&r),
        })
    }
}

impl Hls<'_> {
    /// The body, and where it came from after any redirect.
    async fn download(
        &self,
        url: &str,
        range: Option<(u64, u64)>,
        limit: u64,
    ) -> Result<(Vec<u8>, url::Url), ApiError> {
        let mut req = self.request(Method::Get, url, &[])?.limit(limit);
        if let Some((offset, len)) = range {
            let last = offset.saturating_add(len.max(1) - 1);
            req = req.header("Range", format!("bytes={offset}-{last}"));
        }
        let asked = req.url.clone();
        let resp = self.send(req).await?;
        if !resp.is_success() {
            return Err(self.error(Problem::Status(resp)));
        }
        Ok((resp.body, resp.moved_to.unwrap_or(asked)))
    }
}

/// A playlist, and its address: a live server may move it (to add a session), and
/// its segments are relative to where it ended up.
pub(super) async fn fetch_playlist(
    net: &dyn Transport,
    url: &str,
) -> Result<(String, url::Url), ApiError> {
    let (bytes, at) = Hls(net).download(url, None, MAX_PLAYLIST).await?;
    let text = String::from_utf8(bytes).map_err(|e| ApiError::Decode(e.to_string()))?;
    Ok((text, at))
}

/// A segment, or the `range` of one: (offset, length) in bytes.
pub(super) async fn fetch_bytes(
    net: &dyn Transport,
    url: &str,
    range: Option<(u64, u64)>,
) -> Result<Vec<u8>, ApiError> {
    let mut last = ApiError::Other(String::new());
    // One retry: segments are small and a dropped request is common on mobile links.
    for _ in 0..2 {
        match Hls(net).download(url, range, MAX_SEGMENT).await {
            Ok((b, _)) => return Ok(b),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// The video player's own transport. It is only used on the player's runtime: a
/// pooled connection belongs to the runtime that opened it.
pub(super) fn http() -> &'static Http {
    static NET: OnceLock<Http> = OnceLock::new();
    NET.get_or_init(Http::default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::Failure;
    use crate::net::fake::Fake;

    #[tokio::test]
    async fn segments_get_one_retry() {
        let net = Fake::default()
            .fail(Failure::Unreachable("dropped".into()))
            .reply(200, "seg");
        assert_eq!(
            fetch_bytes(&net, "https://v.example/1.ts", None)
                .await
                .unwrap(),
            b"seg"
        );
        let net = Fake::default().reply(500, "").reply(404, "");
        let err = fetch_bytes(&net, "https://v.example/1.ts", None)
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "the server answered 404 Not Found");
        assert_eq!(net.seen()[0].limit, MAX_SEGMENT);
    }

    #[tokio::test]
    async fn playlists_are_capped() {
        let big = "#".repeat(MAX_PLAYLIST as usize + 1);
        let net = Fake::default().reply(200, &big);
        let err = fetch_playlist(&net, "https://v.example/a.m3u8")
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "The video is larger than expected");
    }

    #[tokio::test]
    async fn parts_of_files_are_asked_for_by_range() {
        let net = Fake::default().reply(206, "part");
        let got = fetch_bytes(&net, "https://v.example/v.mp4", Some((800, 1000)))
            .await
            .unwrap();
        assert_eq!(got, b"part");
        let asked = &net.seen()[0].headers;
        assert!(asked.contains(&("Range".to_string(), "bytes=800-1799".to_string())));
    }
}
