//! Uploading videos. Bluesky's video service takes the file, re-encodes it for
//! streaming and hands back a blob for the post to point at.

use super::client::{ApiClient, Problem, server_message, status_text};
use super::{AnyAgent, ApiError, Client, conv, xrpc_err};
use crate::net::{Method, Transport};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

mod probe;

pub use probe::probe;

const VIDEO_SERVICE: &str = "https://video.bsky.app";
/// The video service's own DID, which the token for asking about limits is made
/// out to.
const VIDEO_DID: &str = "did:web:video.bsky.app";
/// The video service's own DID, which upload tokens are made out to via the PDS.
const UPLOAD_LXM: &str = "com.atproto.repo.uploadBlob";
const LIMITS_LXM: &str = "app.bsky.video.getUploadLimits";
/// Largest file Bluesky takes (since August 2026).
pub const MAX_VIDEO_BYTES: usize = 300 * 1024 * 1024;
/// Longest video Bluesky takes, in seconds (10 minutes since August 2026).
pub const MAX_VIDEO_SECS: f64 = 600.0;

/// A video picked for a post.
#[derive(Clone)]
pub struct DraftVideo {
    pub bytes: Vec<u8>,
    pub name: String,
    pub alt: String,
    pub info: VideoInfo,
}

impl std::fmt::Debug for DraftVideo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DraftVideo({}, {} bytes)", self.name, self.bytes.len())
    }
}

/// What could be read from a video file's header.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VideoInfo {
    /// Picture size as shown, after any rotation.
    pub size: Option<(u32, u32)>,
    pub duration: Option<f64>,
}

/// Whether a file name looks like a video Bluesky takes.
pub fn is_video_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["mp4", "m4v", "mov", "webm", "mpeg", "mpg"]
        .iter()
        .any(|ext| lower.ends_with(&format!(".{ext}")))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobStatus {
    job_id: String,
    state: String,
    blob: Option<Value>,
    error: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobReply {
    job_status: JobStatus,
}

/// What the video service says you may still upload today.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadLimits {
    can_upload: bool,
    remaining_daily_bytes: Option<u64>,
    message: Option<String>,
    error: Option<String>,
}

impl UploadLimits {
    /// Why a video of `bytes` can't be uploaded now, if it can't.
    fn refusal(self, bytes: usize) -> Option<String> {
        if !self.can_upload {
            let why = self.message.or(self.error);
            return Some(why.unwrap_or_else(|| "You can't upload videos right now.".into()));
        }
        match self.remaining_daily_bytes {
            Some(left) if bytes as u64 > left => Some(format!(
                "That video is more than you can upload today ({} MB left). Try again tomorrow.",
                left / 1_048_576
            )),
            _ => None,
        }
    }
}

/// The host of a URL, for a `did:web`.
fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(str::to_string)
}

impl Client {
    /// Uploads a video and waits until Bluesky has processed it, returning the blob
    /// to embed.
    pub async fn upload_video(&self, video: &DraftVideo) -> Result<Value, ApiError> {
        let svc = VideoService(crate::net::shared());
        // Ask first, so a daily limit is explained before a long upload. If the
        // question itself fails, the upload goes ahead and the service decides.
        if let Ok(limits) = self.upload_limits(&svc).await
            && let Some(why) = limits.refusal(video.bytes.len())
        {
            return Err(ApiError::Other(why));
        }

        let session = self.accounts.session().await?;
        let pds = host_of(&session.pds_url)
            .ok_or_else(|| ApiError::Other("can't tell which server your account is on".into()))?;
        let token = self
            .service_token(&format!("did:web:{pds}"), UPLOAD_LXM)
            .await?;
        let req = svc
            .request(
                Method::Post,
                "/xrpc/app.bsky.video.uploadVideo",
                &[
                    ("did", session.did.as_str()),
                    ("name", &upload_name(&video.name)),
                ],
            )?
            .header("Authorization", format!("Bearer {token}"))
            .body(mime_of(&video.name), video.bytes.clone())
            .timeout(Duration::from_mins(10));
        let resp = svc.send(req).await?;
        let body: Value = serde_json::from_slice(&resp.body).unwrap_or(Value::Null);
        // The same file uploaded again is "already_exists", with its job.
        let job = match serde_json::from_value::<JobReply>(body.clone()) {
            Ok(r) => r.job_status,
            Err(_) => match serde_json::from_value::<JobStatus>(body) {
                Ok(j) => j,
                Err(_) => return Err(svc.error(Problem::Status(resp))),
            },
        };
        let blob = self.wait_for_video(job).await?;
        Ok(blob)
    }

    /// A token for one call to a Bluesky service on your behalf, good for 30 minutes.
    async fn service_token(&self, aud: &str, lxm: &str) -> Result<String, ApiError> {
        let exp = chrono::Utc::now().timestamp() + 30 * 60;
        Ok(
            xrpc!(self, |a| a.api.com.atproto.server.get_service_auth(conv(
                serde_json::json!({ "aud": aud, "lxm": lxm, "exp": exp })
            )?))
            .data
            .token
            .clone(),
        )
    }

    async fn upload_limits(&self, svc: &VideoService<'_>) -> Result<UploadLimits, ApiError> {
        let token = self.service_token(VIDEO_DID, LIMITS_LXM).await?;
        let req = svc
            .request(Method::Get, "/xrpc/app.bsky.video.getUploadLimits", &[])?
            .header("Authorization", format!("Bearer {token}"));
        svc.json(req).await
    }

    async fn wait_for_video(&self, mut job: JobStatus) -> Result<Value, ApiError> {
        let svc = VideoService(crate::net::shared());
        // Processing takes about as long as the video; give it plenty.
        for _ in 0..600 {
            if let Some(blob) = job.blob.take() {
                return Ok(blob);
            }
            if job.state == "JOB_STATE_FAILED" {
                let why = job.message.or(job.error).unwrap_or_default();
                return Err(ApiError::Other(format!(
                    "Bluesky couldn't process the video. {why}"
                )));
            }
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let reply: JobReply = svc
                .get(
                    "/xrpc/app.bsky.video.getJobStatus",
                    &[("jobId", job.job_id.as_str())],
                )
                .await?;
            job = reply.job_status;
        }
        Err(ApiError::Other("the video took too long to process".into()))
    }
}

/// Bluesky's video service, which takes uploads and reports on processing.
struct VideoService<'a>(&'a dyn Transport);

impl ApiClient for VideoService<'_> {
    fn base_url(&self) -> &str {
        VIDEO_SERVICE
    }

    fn transport(&self) -> &dyn Transport {
        self.0
    }

    fn error(&self, problem: Problem) -> ApiError {
        ApiError::Other(match problem {
            Problem::Unreachable(e) => format!("couldn't reach Bluesky's video service: {e}"),
            Problem::TooBig => "the video service's reply was too large".into(),
            Problem::Status(r) => match r.status {
                401 | 403 => format!(
                    "video uploads aren't allowed for this account yet: {}",
                    server_message(&r.body).unwrap_or_else(|| "no details".into())
                ),
                413 => "the video is too large".into(),
                _ => format!("video upload failed: {}", status_text(&r)),
            },
        })
    }
}

/// Names sent to the service: just a safe file name.
fn upload_name(name: &str) -> String {
    let n: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .take(60)
        .collect();
    if n.is_empty() { "video.mp4".into() } else { n }
}

fn mime_of(name: &str) -> &'static str {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("mpeg" | "mpg") => "video/mpeg",
        _ => "video/mp4",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn daily_limits_are_explained() {
        let limits = |v: Value| serde_json::from_value::<UploadLimits>(v).unwrap();
        let open = json!({"canUpload": true, "remainingDailyVideos": 20, "remainingDailyBytes": 10_485_760});
        assert_eq!(limits(open.clone()).refusal(1_000), None);
        let over = limits(open).refusal(20_000_000).unwrap();
        assert!(over.contains("10 MB left"), "{over}");
        let shut = json!({"canUpload": false, "message": "Verify your email first."});
        assert_eq!(
            limits(shut).refusal(1).as_deref(),
            Some("Verify your email first.")
        );
    }
}
