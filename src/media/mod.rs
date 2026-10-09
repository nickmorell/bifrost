//! Media files on this computer: preparing images and videos for upload, and picking, pasting
//! or reading the images and videos a post carries.

mod error;
pub mod files;
pub mod image;
pub mod picture;
pub mod video;

pub use error::{FileError, MediaError};
