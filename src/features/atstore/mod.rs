//! ATstore (atstore.fyi): browsing the directory of apps and services built on the AT
//! Protocol, by tag or search, and an app's page with its description, pictures and
//! links. Reading only: nothing here signs in to ATstore or writes to it.
//!
//! The app keeps a [`Store`], hands it the directory it kept on this computer with
//! [`Store::restore`], passes it every [`Msg`] with [`Store::update`], and does what
//! the returned [`Event`]s ask: keep the directory again, load pictures, open pages.
//! The directory comes through [`AtstoreApi`](crate::api::service::AtstoreApi).

mod update;
mod view;

#[cfg(test)]
mod tests;

pub use view::View;

use crate::api::ApiError;
use crate::model::{AtUri, Catalog, Listing, ListingDetail};
use std::collections::HashMap;
use std::fmt;

type Out = crate::features::Update<Msg, Event>;

/// Listings shown at first, and added by each "Show more".
const PAGE: usize = 40;
/// How old the kept directory may get before opening the screen reads it again.
const STALE_HOURS: i64 = 12;

/// The order listings are shown in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Sort {
    /// ATstore's own order.
    #[default]
    Popular,
    /// The best reviewed first.
    Rating,
    Name,
}

impl Sort {
    pub const ALL: [Sort; 3] = [Sort::Popular, Sort::Rating, Sort::Name];
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Sort::Popular => "Popular",
            Sort::Rating => "Top rated",
            Sort::Name => "A to Z",
        })
    }
}

/// The directory and what the ATstore screens show of it.
#[derive(Debug)]
pub struct Store {
    catalog: Catalog,
    loading: bool,
    error: Option<String>,
    query: String,
    tag: Option<String>,
    sort: Sort,
    /// How many of the matching listings are shown.
    shown: usize,
    /// Listing pages being read, or that failed to read.
    pages: HashMap<AtUri, Reading>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            catalog: Catalog::default(),
            loading: false,
            error: None,
            query: String::new(),
            tag: None,
            sort: Sort::default(),
            shown: PAGE,
            pages: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Reading {
    Loading,
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// The ATstore screen is shown: read the directory unless the kept one will do.
    /// `offline` when Bluesky can't be reached; `data_saver` when it is on.
    Open {
        now: String,
        offline: bool,
        data_saver: bool,
    },
    /// Read the directory again.
    Refresh,
    Loaded(Result<Box<Catalog>, ApiError>),
    Query(String),
    /// Show only listings with this tag, or all of them.
    Tag(Option<String>),
    Sort(Sort),
    /// Show more of the matching listings.
    More,
    /// A listing was picked.
    Select(AtUri),
    /// A listing's screen is shown: read its page unless it is kept, or offline.
    Show {
        uri: AtUri,
        offline: bool,
    },
    /// Read a listing's page again.
    Reload(AtUri),
    Page(AtUri, Result<Box<ListingDetail>, ApiError>),
    OpenUrl(String),
    /// Open the Bluesky profile of an app's account, by handle.
    OpenProfile(String),
}

/// What the app has to do for the ATstore screens.
#[derive(Debug)]
pub enum Event {
    /// Load these pictures.
    Pictures(Vec<String>),
    /// Keep the directory on this computer.
    Save(Box<Catalog>),
    OpenListing(AtUri),
    OpenUrl(String),
    OpenProfile(String),
}

impl Store {
    /// Takes the directory kept on this computer, unless one is already here.
    pub fn restore(&mut self, kept: Option<Catalog>) {
        if let Some(c) = kept.filter(|_| self.catalog.listings.is_empty()) {
            self.catalog = c;
        }
    }

    /// Whether the directory has been read or restored.
    pub fn has_catalog(&self) -> bool {
        !self.catalog.listings.is_empty()
    }

    /// The name to show for a listing's screen.
    pub fn title(&self, uri: &AtUri) -> String {
        self.listing(uri)
            .map_or_else(|| "ATstore".to_string(), |l| l.name.clone())
    }

    fn listing(&self, uri: &AtUri) -> Option<&Listing> {
        self.catalog
            .details
            .get(uri)
            .map(|d| &d.listing)
            .or_else(|| self.catalog.listing(uri))
    }

    /// The listings that match the search and tag, in the chosen order.
    fn matching(&self) -> Vec<&Listing> {
        let mut found: Vec<&Listing> = self
            .catalog
            .listings
            .iter()
            .filter(|l| self.tag.as_ref().is_none_or(|t| l.app_tags.contains(t)))
            .filter(|l| l.matches(&self.query))
            .collect();
        match self.sort {
            Sort::Popular => {}
            Sort::Rating => found.sort_by(|a, b| {
                let stars = |l: &Listing| l.stars().unwrap_or(0.0);
                stars(b)
                    .total_cmp(&stars(a))
                    .then(b.review_count.cmp(&a.review_count))
            }),
            Sort::Name => found.sort_by_key(|l| l.name.to_lowercase()),
        }
        found
    }
}
