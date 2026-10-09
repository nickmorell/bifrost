//! Deck columns: what each column of Home in deck mode shows, and how wide they are.

use super::{AtUri, Did, FeedSource};
use serde::{Deserialize, Serialize};

/// One column of the deck.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DeckColumn {
    Following,
    Feed { uri: AtUri, name: String },
    List { uri: AtUri, name: String },
    Notifications,
    Bookmarks,
    Profile { did: Did, name: String },
}

impl DeckColumn {
    pub fn title(&self) -> &str {
        match self {
            DeckColumn::Following => "Following",
            DeckColumn::Feed { name, .. }
            | DeckColumn::List { name, .. }
            | DeckColumn::Profile { name, .. } => name,
            DeckColumn::Notifications => "Notifications",
            DeckColumn::Bookmarks => "Bookmarks",
        }
    }

    /// The column showing a feed or list.
    pub fn for_source(source: &FeedSource, name: &str) -> Self {
        match source {
            FeedSource::Following => DeckColumn::Following,
            FeedSource::Generator(uri) => DeckColumn::Feed {
                uri: uri.clone(),
                name: name.to_string(),
            },
            FeedSource::List(uri) => DeckColumn::List {
                uri: uri.clone(),
                name: name.to_string(),
            },
        }
    }
}

/// How wide deck columns are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeckWidth {
    Narrow,
    #[default]
    Normal,
    Wide,
}

impl DeckWidth {
    pub const ALL: [DeckWidth; 3] = [DeckWidth::Narrow, DeckWidth::Normal, DeckWidth::Wide];

    pub fn pixels(self) -> f32 {
        match self {
            DeckWidth::Narrow => 340.0,
            DeckWidth::Normal => 410.0,
            DeckWidth::Wide => 500.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DeckWidth::Narrow => "Narrow",
            DeckWidth::Normal => "Normal",
            DeckWidth::Wide => "Wide",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_round_trip() {
        let cols = vec![
            DeckColumn::Following,
            DeckColumn::Feed {
                uri: "at://did:plc:a/app.bsky.feed.generator/x".into(),
                name: "Cats".into(),
            },
            DeckColumn::Notifications,
        ];
        let json = serde_json::to_string(&cols).unwrap();
        assert!(json.contains("\"kind\":\"feed\""));
        let back: Vec<DeckColumn> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cols);
        assert_eq!(back[1].title(), "Cats");
    }
}
