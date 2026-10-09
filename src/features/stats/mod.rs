//! Your stats: what your posts of the last month did, worked out on this computer
//! from your posts kept here. Opening the screen reads your latest posts again so
//! the counts are fresh; the app keeps them and hands the [`Stats`] to [`View`].

mod view;

#[cfg(test)]
mod tests;

pub use view::{Host, View};

use crate::api::ApiError;
use crate::api::service::StatsApi;
use crate::features::Update;
use crate::model::PostView;
use iced::Task;

/// Whether your latest posts are being read again.
#[derive(Debug, Default)]
pub struct Screen {
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Read the latest posts of the account with this DID.
    Load(String),
    Loaded(Result<Vec<PostView>, ApiError>),
}

#[derive(Debug)]
pub enum Event {
    /// Keep these posts of yours, so the stats count them.
    Posts(Vec<PostView>),
}

impl Screen {
    pub fn update(&mut self, msg: Msg, api: &impl StatsApi) -> Update<Msg, Event> {
        match msg {
            Msg::Load(did) => {
                if self.loading {
                    return Update::none();
                }
                self.loading = true;
                self.error = None;
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.my_posts(&did).await },
                    Msg::Loaded,
                ))
            }
            Msg::Loaded(r) => {
                self.loading = false;
                match r {
                    Ok(posts) => Update::event(Event::Posts(posts)),
                    Err(e) => {
                        self.error = Some(e.friendly());
                        Update::none()
                    }
                }
            }
        }
    }
}
