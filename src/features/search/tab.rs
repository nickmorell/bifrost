//! The result tabs of a search.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Top,
    Latest,
    People,
    Feeds,
    /// Posts kept on this computer.
    Saved,
}

impl Tab {
    /// The `sort` value searchPosts takes for the post tabs.
    pub(super) fn sort(self) -> &'static str {
        match self {
            Tab::Latest => "latest",
            _ => "top",
        }
    }
}
