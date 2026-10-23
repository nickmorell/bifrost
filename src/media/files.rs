//! Getting the images and video BiFrost uploads: picked in a file dialog, pasted from
//! the clipboard or dropped on the window. [`Attachments`] is the black box the
//! composer and the profile editor use; [`Local`] is the real one, and tests use a
//! fake.

use super::image::{Prepared, is_image_path, prepare_cropped, prepare_image};
use super::{FileError, MediaError};
use crate::api::{self, DraftVideo};
use crate::platform::clipboard::{self, ClipboardImages};
use std::future::Future;
use std::path::PathBuf;

/// Images found, each prepared for upload or with why it could not be.
pub type Images = Vec<Result<Prepared, FileError>>;
/// A video, or why it can't be used; `None` when nothing was picked.
pub type Video = Option<Result<DraftVideo, FileError>>;

/// Where uploaded images and video come from. Each call returns a future that can
/// move to another thread, so it can run as an iced task.
pub trait Attachments: Clone + Send + Sync + 'static {
    /// Asks for up to `max` images in a file dialog.
    fn pick_images(&self, max: usize) -> impl Future<Output = Images> + Send;
    /// Asks for one video in a file dialog.
    fn pick_video(&self) -> impl Future<Output = Video> + Send;
    /// Reads up to `max` images from these files, skipping anything else.
    fn load_images(&self, paths: Vec<PathBuf>, max: usize) -> impl Future<Output = Images> + Send;
    /// Reads a video file.
    fn load_video(&self, path: PathBuf) -> impl Future<Output = Video> + Send;
    /// Images copied to the clipboard; empty when it holds none.
    fn clipboard_images(&self, max: usize) -> impl Future<Output = Images> + Send;
    /// Asks for one image in a file dialog titled `title`, cropped to `aspect` (width
    /// over height) and at most `max_width` wide. `None` when nothing was picked.
    fn pick_cropped(
        &self,
        title: &'static str,
        aspect: f32,
        max_width: u32,
    ) -> impl Future<Output = Option<Result<Prepared, MediaError>>> + Send;
}

/// Files on this computer and its clipboard.
#[derive(Debug, Clone, Copy, Default)]
pub struct Local;

impl Attachments for Local {
    fn pick_images(&self, max: usize) -> impl Future<Output = Images> + Send {
        pick_images(max)
    }
    fn pick_video(&self) -> impl Future<Output = Video> + Send {
        pick_video()
    }
    fn load_images(&self, paths: Vec<PathBuf>, max: usize) -> impl Future<Output = Images> + Send {
        load_images(paths, max)
    }
    fn load_video(&self, path: PathBuf) -> impl Future<Output = Video> + Send {
        load_video(path)
    }
    fn clipboard_images(&self, max: usize) -> impl Future<Output = Images> + Send {
        clipboard_images(max)
    }
    fn pick_cropped(
        &self,
        title: &'static str,
        aspect: f32,
        max_width: u32,
    ) -> impl Future<Output = Option<Result<Prepared, MediaError>>> + Send {
        pick_cropped(title, aspect, max_width)
    }
}

/// Whether a file looks like a video Bluesky takes.
pub fn is_video_path(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(api::is_video_name)
}

/// A picture (for example a screenshot) or image files copied in the file manager.
async fn clipboard_images(max: usize) -> Images {
    let found = tokio::task::spawn_blocking(clipboard::images)
        .await
        .ok()
        .flatten();
    match found {
        Some(ClipboardImages::Picture(bmp)) => {
            vec![prepare_image(bmp).map_err(|e| e.in_file("Pasted image"))]
        }
        Some(ClipboardImages::Files(files)) => load_images(files, max).await,
        None => Vec::new(),
    }
}

/// Reads a video and checks it against Bluesky's limits.
fn prepare_video(name: String, mut bytes: Vec<u8>) -> Result<DraftVideo, FileError> {
    if bytes.len() > api::MAX_VIDEO_BYTES {
        let mb = (bytes.len() as f64 / 1_048_576.0).round() as u64;
        return Err(MediaError::VideoTooBig(mb).in_file(name));
    }
    super::video::strip(&mut bytes);
    let info = api::probe_video(&bytes);
    if info.duration.is_some_and(|d| d > api::MAX_VIDEO_SECS + 0.5) {
        return Err(MediaError::VideoTooLong.in_file(name));
    }
    Ok(DraftVideo {
        bytes,
        name,
        alt: String::new(),
        info,
    })
}

async fn load_video(path: PathBuf) -> Video {
    let name = path
        .file_name()
        .map_or_else(|| "video.mp4".into(), |n| n.to_string_lossy().into_owned());
    Some(match tokio::fs::read(&path).await {
        Ok(bytes) => prepare_video(name, bytes),
        Err(e) => Err(MediaError::Read(e.to_string()).in_file(name)),
    })
}

async fn pick_video() -> Video {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("Videos", &["mp4", "m4v", "mov", "webm", "mpeg", "mpg"])
        .set_title("Add a video")
        .pick_file()
        .await?;
    let name = file.file_name();
    let bytes = file.read().await;
    Some(prepare_video(name, bytes))
}

async fn load_images(paths: Vec<PathBuf>, max: usize) -> Images {
    let mut out = Vec::new();
    for path in paths.into_iter().filter(|p| is_image_path(p)).take(max) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let result = match tokio::fs::read(&path).await {
            Ok(bytes) => prepare_off_thread(bytes).await,
            Err(e) => Err(MediaError::Read(e.to_string())),
        };
        out.push(result.map_err(|e| e.in_file(name)));
    }
    out
}

/// Prepares an image for upload away from the window's thread.
async fn prepare_off_thread(bytes: Vec<u8>) -> Result<Prepared, MediaError> {
    tokio::task::spawn_blocking(move || prepare_image(bytes))
        .await
        .map_err(|e| MediaError::Read(e.to_string()))
        .and_then(|r| r)
}

async fn pick_images(max: usize) -> Images {
    let Some(files) = rfd::AsyncFileDialog::new()
        .add_filter("Images", &["jpg", "jpeg", "png", "webp", "gif"])
        .set_title("Add images")
        .pick_files()
        .await
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for f in files.into_iter().take(max) {
        let bytes = f.read().await;
        let name = f.file_name();
        out.push(prepare_off_thread(bytes).await.map_err(|e| e.in_file(name)));
    }
    out
}

async fn pick_cropped(
    title: &'static str,
    aspect: f32,
    max_width: u32,
) -> Option<Result<Prepared, MediaError>> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("Images", &["jpg", "jpeg", "png", "webp", "gif", "bmp"])
        .set_title(title)
        .pick_file()
        .await?;
    let bytes = file.read().await;
    Some(
        tokio::task::spawn_blocking(move || prepare_cropped(bytes, aspect, max_width))
            .await
            .map_err(|e| MediaError::Read(e.to_string()))
            .and_then(|r| r),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn file_kinds() {
        assert!(is_image_path(Path::new("a/B.JPG")));
        assert!(!is_image_path(Path::new("notes.txt")));
        assert!(is_video_path(Path::new("clip.MOV")));
        assert!(!is_video_path(Path::new("clip.gif")));
    }

    #[test]
    fn videos_over_the_limit_are_refused() {
        let big = vec![0; api::MAX_VIDEO_BYTES + 1];
        let err = prepare_video("big.mp4".into(), big).unwrap_err();
        assert!(err.to_string().starts_with("big.mp4 is 300 MB"), "{err}");
        let ok = prepare_video("small.mp4".into(), vec![0; 10]).unwrap();
        assert_eq!(ok.name, "small.mp4");
    }
}
