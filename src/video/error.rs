//! Why a video can't play.

use crate::api::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlayError {
    /// Fetching a playlist or segment failed.
    #[error("{}", .0.friendly())]
    Network(#[from] ApiError),
    #[error("The video has no streams")]
    NoStreams,
    #[error("The video is empty")]
    Empty,
    #[error("The stream hasn't started yet")]
    NotStarted,
    #[error("This video's format can't be played here")]
    Unsupported,
    /// A thread or runtime the player needs couldn't start.
    #[error("Couldn't start the {0}")]
    NoThread(&'static str),
    #[error("Couldn't decode the video ({0})")]
    Decode(String),
}
