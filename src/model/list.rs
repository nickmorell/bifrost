//! Lists (curated and moderation) and starter packs.

use super::{AtUri, Author, FeedGenerator, Page};
use serde::Deserialize;

/// A list as shown on a profile.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListInfo {
    pub uri: AtUri,
    pub name: String,
    /// "app.bsky.graph.defs#curatelist", "#modlist" or "#referencelist".
    #[serde(default)]
    pub purpose: String,
    pub description: Option<String>,
    pub avatar: Option<String>,
}

pub const CURATE: &str = "app.bsky.graph.defs#curatelist";
pub const MODLIST: &str = "app.bsky.graph.defs#modlist";
/// The kind of list a starter pack keeps its people in.
pub const REFERENCE: &str = "app.bsky.graph.defs#referencelist";
/// Most people a starter pack can hold.
pub const MAX_PACK_PEOPLE: usize = 150;
/// Most feeds a starter pack can hold.
pub const MAX_PACK_FEEDS: usize = 3;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListViewer {
    #[serde(default)]
    pub muted: bool,
    /// Your listblock record, when you block the list.
    pub blocked: Option<AtUri>,
}

/// A list with its details.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListView {
    pub uri: AtUri,
    #[serde(default)]
    pub cid: String,
    pub name: String,
    #[serde(default)]
    pub purpose: String,
    pub description: Option<String>,
    pub avatar: Option<String>,
    pub creator: Author,
    #[serde(default)]
    pub list_item_count: Option<u64>,
    #[serde(default)]
    pub viewer: ListViewer,
}

impl ListView {
    pub fn is_mod(&self) -> bool {
        self.purpose == MODLIST
    }

    pub fn kind_label(&self) -> &'static str {
        match self.purpose.as_str() {
            MODLIST => "Moderation list",
            REFERENCE => "Starter pack list",
            _ => "List",
        }
    }
}

/// One person on a list, with the record that put them there.
#[derive(Debug, Clone, Deserialize)]
pub struct ListItem {
    pub uri: AtUri,
    pub subject: Author,
}

#[derive(Debug, Clone)]
pub struct ListPage {
    pub list: ListView,
    pub items: Page<ListItem>,
}

/// The parts of a starter pack's record BiFrost shows and edits.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StarterPackRecord {
    #[serde(default)]
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub list: AtUri,
    #[serde(default)]
    pub feeds: Vec<FeedRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedRef {
    pub uri: AtUri,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBasic {
    pub uri: AtUri,
    #[serde(default)]
    pub list_item_count: Option<u64>,
}

/// A starter pack: someone's suggested people and feeds to follow.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StarterPack {
    pub uri: AtUri,
    #[serde(default)]
    pub cid: String,
    pub record: StarterPackRecord,
    pub creator: Author,
    pub list: Option<ListBasic>,
    #[serde(default)]
    pub feeds: Vec<FeedGenerator>,
    #[serde(default)]
    pub joined_all_time_count: Option<u64>,
    #[serde(default)]
    pub joined_week_count: Option<u64>,
}

/// A starter pack as listed on a profile.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StarterPackBasic {
    pub uri: AtUri,
    pub record: StarterPackRecord,
    pub creator: Author,
    #[serde(default)]
    pub list_item_count: Option<u64>,
}

/// One of your lists and, if the person is on it, the record that put them there.
#[derive(Debug, Clone)]
pub struct Membership {
    pub list: ListView,
    pub item: Option<AtUri>,
}

/// A new list or starter pack, or new details for one.
#[derive(Debug, Clone, Default)]
pub struct ListDetails {
    pub name: String,
    pub description: String,
    /// For a new list: [`CURATE`] or [`MODLIST`].
    pub purpose: String,
}
