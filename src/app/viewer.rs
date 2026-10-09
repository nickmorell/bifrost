//! The full-window image viewer.

use super::{App, Message, media};
use crate::api;
use crate::media::MediaError;
use iced::Task;

#[derive(Debug, Clone)]
pub enum Msg {
    /// Open the image viewer on these images at this index.
    Open(Vec<media::ViewerImage>, usize),
    Step(isize),
    Close,
    Save,
    Saved(Result<Option<String>, MediaError>),
}

impl App {
    pub(super) fn viewer_update(&mut self, msg: Msg) -> Task<Message> {
        match msg {
            Msg::Open(images, index) => {
                if images.is_empty() {
                    return Task::none();
                }
                let urls: Vec<String> = images.iter().map(|i| i.full.clone()).collect();
                self.viewer = Some(media::Viewer {
                    index: index.min(images.len() - 1),
                    images,
                });
                self.request_images(urls)
            }
            Msg::Step(step) => {
                if let Some(v) = &mut self.viewer {
                    v.index = v.index.saturating_add_signed(step).min(v.images.len() - 1);
                }
                Task::none()
            }
            Msg::Close => {
                self.viewer = None;
                Task::none()
            }
            Msg::Save => {
                let Some(v) = &self.viewer else {
                    return Task::none();
                };
                let url = v.current().full.clone();
                let name = media::file_name(&url);
                Task::perform(
                    async move {
                        let bytes = api::fetch_bytes(&url)
                            .await
                            .map_err(|e| MediaError::Read(e.friendly()))?;
                        let Some(file) = rfd::AsyncFileDialog::new()
                            .set_file_name(&name)
                            .save_file()
                            .await
                        else {
                            return Ok(None);
                        };
                        tokio::fs::write(file.path(), bytes)
                            .await
                            .map_err(|e| MediaError::Read(e.to_string()))?;
                        Ok(Some(file.path().display().to_string()))
                    },
                    |a| Message::Viewer(Msg::Saved(a)),
                )
            }
            Msg::Saved(result) => {
                match result {
                    Ok(Some(path)) => self.toast = Some(format!("Saved to {path}")),
                    Ok(None) => {}
                    Err(e) => self.toast = Some(format!("Couldn't save the image. {e}")),
                }
                Task::none()
            }
        }
    }
}
