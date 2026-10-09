//! Images and video: picked, pasted or dropped, with their alt text.

use super::Out;
use super::message::Msg;
use super::state::{Attached, Composer, MAX_IMAGES};
use crate::features::Update;
use crate::media::files::{Attachments, is_video_path};
use iced::Task;

const ONE_VIDEO: &str = "A post can have one video, without images.";

impl Composer {
    /// Handles the messages about images and video.
    pub(super) fn attach_update<F: Attachments>(&mut self, msg: Msg, files: &F) -> Out {
        let remaining = MAX_IMAGES.saturating_sub(self.images.len());
        match msg {
            Msg::Paste => {
                let files = files.clone();
                Update::task(Task::perform(
                    async move { files.clipboard_images(remaining).await },
                    Msg::Pasted,
                ))
            }
            Msg::Pasted(images) if images.is_empty() => {
                Update::task(iced::clipboard::read().map(Msg::PasteText))
            }
            Msg::Pasted(images) => self.attach_update(Msg::ImagesPicked(images), files),
            Msg::Dropped(paths) if paths.iter().any(|p| is_video_path(p)) => {
                if self.has_media() {
                    self.error = Some(ONE_VIDEO.into());
                    return Update::none();
                }
                let Some(path) = paths.into_iter().find(|p| is_video_path(p)) else {
                    return Update::none();
                };
                let files = files.clone();
                Update::task(Task::perform(
                    async move { files.load_video(path).await },
                    Msg::VideoPicked,
                ))
            }
            Msg::Dropped(_) if self.video.is_some() => {
                self.error = Some(ONE_VIDEO.into());
                Update::none()
            }
            Msg::Dropped(paths) => {
                if remaining == 0 {
                    self.error = Some(format!("A post can have up to {MAX_IMAGES} images."));
                    return Update::none();
                }
                let files = files.clone();
                Update::task(Task::perform(
                    async move { files.load_images(paths, remaining).await },
                    Msg::ImagesPicked,
                ))
            }
            Msg::AddImages if remaining == 0 => Update::none(),
            Msg::AddImages => {
                let files = files.clone();
                Update::task(Task::perform(
                    async move { files.pick_images(remaining).await },
                    Msg::ImagesPicked,
                ))
            }
            Msg::ImagesPicked(results) if self.video.is_some() && !results.is_empty() => {
                self.error = Some(ONE_VIDEO.into());
                Update::none()
            }
            Msg::ImagesPicked(results) => {
                for r in results {
                    match r {
                        Ok(img) if self.images.len() < MAX_IMAGES => {
                            self.images.push(Attached::new(img, String::new()));
                        }
                        Ok(_) => {}
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
                Update::none()
            }
            Msg::RemoveImage(i) => {
                if i < self.images.len() {
                    self.images.remove(i);
                }
                Update::none()
            }
            Msg::Alt(i, alt) => {
                if let Some(a) = self.images.get_mut(i) {
                    a.alt = alt.chars().take(2000).collect();
                }
                Update::none()
            }
            Msg::AddVideo if self.has_media() => Update::none(),
            Msg::AddVideo => {
                let files = files.clone();
                Update::task(Task::perform(
                    async move { files.pick_video().await },
                    Msg::VideoPicked,
                ))
            }
            Msg::VideoPicked(Some(Ok(v))) if !self.has_media() => {
                self.error = None;
                self.video = Some(v);
                Update::none()
            }
            Msg::VideoPicked(Some(Err(e))) => {
                self.error = Some(e.to_string());
                Update::none()
            }
            Msg::RemoveVideo => {
                self.video = None;
                Update::none()
            }
            Msg::VideoAlt(alt) => {
                if let Some(v) = &mut self.video {
                    // Bluesky's limit for video alt text.
                    v.alt = alt.chars().take(1000).collect();
                }
                Update::none()
            }
            // A video picked after media was added, nothing picked, or a message
            // handled in `update`.
            _ => Update::none(),
        }
    }
}
