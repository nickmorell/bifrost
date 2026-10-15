//! What BiFrost keeps on this computer: settings, drafts, secrets in the OS credential
//! store, where you were reading in each feed, the ATstore directory and the image
//! cache. Folders come from [`crate::platform::paths`].

pub mod archive;
pub mod atstore;
pub mod collections;
pub mod drafts;
pub mod image_cache;
mod later;
pub mod places;
pub mod secrets;
pub mod settings;

pub use later::{Failed, failures};
