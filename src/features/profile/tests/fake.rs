//! Fakes for the profile tests. The tests check what `update` does with each message
//! and how many loads it starts, so the calls are never run and answer nothing.

use crate::api::service::ProfileApi;
use crate::api::{ApiError, ProfileEdit};
use crate::media::files::{Attachments, Images, Video};
use crate::media::image::Prepared;
use crate::model::{FeedGenerator, ListInfo, Profile};
use std::future::{Ready, ready};
use std::path::PathBuf;

#[derive(Clone, Default)]
pub struct Fake;

fn nothing<T>() -> Ready<Result<T, ApiError>> {
    ready(Err(ApiError::Other("not run in tests".into())))
}

impl ProfileApi for Fake {
    fn profile(&self, _: &str) -> impl Future<Output = Result<Profile, ApiError>> + Send {
        nothing()
    }
    fn actor_feeds(
        &self,
        _: &str,
    ) -> impl Future<Output = Result<Vec<FeedGenerator>, ApiError>> + Send {
        nothing()
    }
    fn actor_lists(&self, _: &str) -> impl Future<Output = Result<Vec<ListInfo>, ApiError>> + Send {
        nothing()
    }
    fn save_profile(&self, _: ProfileEdit) -> impl Future<Output = Result<(), ApiError>> + Send {
        nothing()
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
