//! Fakes for the composer's tests. The tests check what `update` does with each
//! message, so the calls are never run and the fakes answer nothing.

use crate::api::service::ComposerApi;
use crate::api::{ApiError, LinkCard, PostDraft};
use crate::media::files::{Attachments, Images, Video};
use crate::media::image::Prepared;
use crate::model::{AtUri, Author};
use std::future::{Ready, ready};
use std::path::PathBuf;

#[derive(Clone, Default)]
pub struct Fake;

fn none<T>() -> Ready<Result<T, ApiError>> {
    ready(Err(ApiError::Other("not in the fake".into())))
}

impl ComposerApi for Fake {
    fn typeahead(&self, _: &str) -> impl Future<Output = Result<Vec<Author>, ApiError>> + Send {
        none()
    }
    fn link_card(&self, _: &str) -> impl Future<Output = Result<LinkCard, ApiError>> + Send {
        none()
    }
    fn create_post(&self, _: PostDraft) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        none()
    }
    fn create_thread(
        &self,
        _: Vec<PostDraft>,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        none()
    }
}

impl Attachments for Fake {
    fn pick_images(&self, _: usize) -> impl Future<Output = Images> + Send {
        ready(Vec::new())
    }
    fn pick_video(&self) -> impl Future<Output = Video> + Send {
        ready(None)
    }
    fn load_images(&self, _: Vec<PathBuf>, _: usize) -> impl Future<Output = Images> + Send {
        ready(Vec::new())
    }
    fn load_video(&self, _: PathBuf) -> impl Future<Output = Video> + Send {
        ready(None)
    }
    fn clipboard_images(&self, _: usize) -> impl Future<Output = Images> + Send {
        ready(Vec::new())
    }
    fn pick_cropped(
        &self,
        _: &'static str,
        _: f32,
        _: u32,
    ) -> impl Future<Output = Option<Result<Prepared, crate::media::MediaError>>> + Send {
        ready(None)
    }
}
