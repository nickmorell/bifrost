//! Uploading videos. Bluesky's video service takes the file, re-encodes it for
//! streaming and hands back a blob for the post to point at.

use super::client::{ApiClient, Problem, server_message, status_text};
use super::{AnyAgent, ApiError, Client, conv, xrpc_err};
use crate::net::{Method, Transport};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

const VIDEO_SERVICE: &str = "https://video.bsky.app";
/// The video service's own DID, which upload tokens are made out to via the PDS.
const UPLOAD_LXM: &str = "com.atproto.repo.uploadBlob";
/// Largest file Bluesky takes.
pub const MAX_VIDEO_BYTES: usize = 100 * 1024 * 1024;
/// Longest video Bluesky takes, in seconds.
pub const MAX_VIDEO_SECS: f64 = 180.0;

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

/// Reads the size and length from an MP4 or QuickTime file. Anything else gives
/// an empty [`VideoInfo`].
pub fn probe(bytes: &[u8]) -> VideoInfo {
    let mut info = VideoInfo::default();
    let Some(moov) = find_box(bytes, *b"moov") else {
        return info;
    };
    if let Some(mvhd) = find_box(moov, *b"mvhd") {
        info.duration = read_mvhd(mvhd);
    }
    for trak in boxes(moov)
        .filter(|(kind, _)| kind == b"trak")
        .map(|(_, b)| b)
    {
        let is_video = find_box(trak, *b"mdia")
            .and_then(|m| find_box(m, *b"hdlr"))
            .is_some_and(|h| h.get(8..12) == Some(b"vide"));
        if let Some(tkhd) = find_box(trak, *b"tkhd")
            && is_video
            && let Some(size) = read_tkhd(tkhd)
        {
            info.size = Some(size);
            break;
        }
    }
    info
}

/// The boxes directly inside `data`: (type, contents).
fn boxes(data: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    let mut pos = 0usize;
    std::iter::from_fn(move || {
        let head = data.get(pos..pos + 8)?;
        let mut size = u32::from_be_bytes(head[0..4].try_into().ok()?) as usize;
        let kind: [u8; 4] = head[4..8].try_into().ok()?;
        let mut header = 8;
        if size == 1 {
            size = u64::from_be_bytes(data.get(pos + 8..pos + 16)?.try_into().ok()?) as usize;
            header = 16;
        } else if size == 0 {
            size = data.len() - pos;
        }
        if size < header {
            return None;
        }
        let body = data.get(pos + header..pos.checked_add(size)?)?;
        pos += size;
        Some((kind, body))
    })
}

fn find_box(data: &[u8], kind: [u8; 4]) -> Option<&[u8]> {
    boxes(data).find(|(k, _)| *k == kind).map(|(_, b)| b)
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

fn read_mvhd(b: &[u8]) -> Option<f64> {
    let (scale, duration) = if *b.first()? == 1 {
        (be32(b, 20)?, be64(b, 24)?)
    } else {
        (be32(b, 12)?, u64::from(be32(b, 16)?))
    };
    (scale > 0).then(|| duration as f64 / f64::from(scale))
}

fn read_tkhd(b: &[u8]) -> Option<(u32, u32)> {
    // The matrix starts after the version-dependent times.
    let matrix = if *b.first()? == 1 { 52 } else { 40 };
    let w = be32(b, matrix + 36)? >> 16;
    let h = be32(b, matrix + 40)? >> 16;
    if w == 0 || h == 0 {
        return None;
    }
    // A matrix with a non-zero `b` turns the picture a quarter, as phones record
    // portrait video.
    let turned = be32(b, matrix + 4)? != 0;
    Some(if turned { (h, w) } else { (w, h) })
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

/// The host of a URL, for a `did:web`.
fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(str::to_string)
}

impl Client {
    /// Uploads a video and waits until Bluesky has processed it, returning the blob
    /// to embed.
    pub async fn upload_video(&self, video: &DraftVideo) -> Result<Value, ApiError> {
        let session = self.accounts.session().await?;
        let pds = host_of(&session.pds_url)
            .ok_or_else(|| ApiError::Other("can't tell which server your account is on".into()))?;
        let aud = format!("did:web:{pds}");
        let exp = chrono::Utc::now().timestamp() + 30 * 60;
        let token = xrpc!(self, |a| a.api.com.atproto.server.get_service_auth(conv(
            serde_json::json!({ "aud": aud, "lxm": UPLOAD_LXM, "exp": exp })
        )?))
        .data
        .token;

        let svc = VideoService(crate::net::shared());
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

    fn mp4_box(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
        let mut b = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        b.extend_from_slice(&kind);
        b.extend_from_slice(body);
        b
    }

    fn tkhd(w: u32, h: u32, turned: bool) -> Vec<u8> {
        let mut b = vec![0u8; 40];
        let mut matrix = [0u32; 9];
        if turned {
            matrix[1] = 0x0001_0000;
            matrix[3] = 0xffff_0000;
        } else {
            matrix[0] = 0x0001_0000;
            matrix[4] = 0x0001_0000;
        }
        matrix[8] = 0x4000_0000;
        for m in matrix {
            b.extend_from_slice(&m.to_be_bytes());
        }
        b.extend_from_slice(&(w << 16).to_be_bytes());
        b.extend_from_slice(&(h << 16).to_be_bytes());
        mp4_box(*b"tkhd", &b)
    }

    fn trak(kind: [u8; 4], w: u32, h: u32, turned: bool) -> Vec<u8> {
        let mut hdlr = vec![0u8; 8];
        hdlr.extend_from_slice(&kind);
        hdlr.extend_from_slice(&[0; 12]);
        let mdia = mp4_box(*b"mdia", &mp4_box(*b"hdlr", &hdlr));
        let mut body = tkhd(w, h, turned);
        body.extend(mdia);
        mp4_box(*b"trak", &body)
    }

    #[test]
    fn reads_size_and_length() {
        let mut mvhd = vec![0u8; 12];
        mvhd.extend_from_slice(&1000u32.to_be_bytes());
        mvhd.extend_from_slice(&12_500u32.to_be_bytes());
        mvhd.extend_from_slice(&[0; 80]);
        let mut moov = mp4_box(*b"mvhd", &mvhd);
        moov.extend(trak(*b"soun", 0, 0, false));
        moov.extend(trak(*b"vide", 1920, 1080, true));
        let mut file = mp4_box(*b"ftyp", b"isom\0\0\0\0");
        file.extend(mp4_box(*b"moov", &moov));
        let info = probe(&file);
        assert_eq!(info.duration, Some(12.5));
        // Recorded on its side, so shown portrait.
        assert_eq!(info.size, Some((1080, 1920)));
        assert_eq!(probe(b"not a video"), VideoInfo::default());
        assert!(is_video_name("Clip.MOV"));
        assert!(!is_video_name("cat.gif"));
        assert_eq!(upload_name("my clip (1).mp4"), "myclip1.mp4");
    }
}
