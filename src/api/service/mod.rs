//! What each feature needs from Bluesky, as one trait per feature.
//!
//! [`Client`](super::Client) implements every trait by calling its own methods. A
//! feature is written against its trait, so its tests can use a fake instead of the
//! network.

mod actions;
mod alerts;
mod articles;
mod atstore;
mod chat;
mod collections;
mod composer;
mod drafts;
mod feeds;
mod lists;
mod live;
mod moderation;
mod notifications;
mod people;
mod prefs;
mod privacy;
mod profile;
mod search;
mod stats;
mod thread;
mod timeline;
mod translate;
pub use actions::ActionsApi;
pub use alerts::AlertsApi;
pub use articles::ArticlesApi;
pub use atstore::AtstoreApi;
pub use chat::ChatApi;
pub use collections::CollectionsApi;
pub use composer::ComposerApi;
pub use drafts::DraftsApi;
pub use feeds::FeedsApi;
pub use lists::ListsApi;
pub use live::{Channel, LiveApi};
pub use moderation::ModerationApi;
pub use notifications::NotificationsApi;
pub use people::PeopleApi;
pub use prefs::PrefsApi;
pub use privacy::PrivacyApi;
pub use profile::ProfileApi;
pub use search::SearchApi;
pub use stats::StatsApi;
pub use thread::ThreadApi;
pub use timeline::TimelineApi;
pub use translate::TranslateApi;
