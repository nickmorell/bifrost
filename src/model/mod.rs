//! BiFrost's own view of Bluesky data: plain types with no I/O.
//!
//! These mirror the lexicon JSON (camelCase, `$type` tags) but only keep what the UI
//! uses. Unknown variants deserialize to an `Other` case instead of failing, so a new
//! embed or record type on the network never breaks a whole timeline page.
//!
//! Every other module may use this one; it uses none of them.

mod actor;
mod alert;
mod article;
mod atstore;
mod chat;
mod collection;
mod common;
mod deck;
mod draft;
mod embed;
mod feed;
mod filter;
mod gate;
mod ids;
mod kept;
mod lang;
mod list;
mod live;
mod notification;
mod post;
mod prefs;
mod privacy;
mod translation;

pub use actor::{ActorViewer, Associated, Author, Germ, PeopleKind, Profile, ProfileLink};
pub use alert::{AlertKind, Alerts};
pub use article::{
    Article, ArticleRef, ArticleSummary, Block, DOCUMENT, PUBLICATION, Publication, SUBSCRIPTION,
    WHITEWIND, blocks,
};
pub use atstore::{Catalog, Listing, ListingDetail, ListingLink};
pub use chat::{AllowIncoming, ChatMessage, Convo, Reaction, Sender};
pub use collection::{Collection, collection_name};
pub use common::{Label, Mine, Page, ReplyRef, StrongRef, tid};
pub use deck::{DeckColumn, DeckWidth};
pub use draft::{Draft, DraftImageFile};
pub use embed::{
    AspectRatio, Embed, EmbeddedRecord, ExternalSource, ExternalView, ImageView, NamedRecord,
    QuotedPost, RecordWrapper, Rgb, SourceTheme,
};
pub use feed::{
    FeedGenerator, FeedItem, FeedReason, FeedReplyContext, MaybePost, ThreadNode, ThreadPost,
    Trend, TrendTarget,
};
pub use filter::{FeedFilter, MIN_LIKES_STEPS};
pub use gate::{Interaction, ReplyRules, ThreadgateView};
pub use ids::{AtUri, Did};
pub use kept::{Kept, KeptLink, Query, STATS_DAYS, Stats, TopLink, domain_of, top_links};
pub use lang::{LANGUAGES, Language, language};
pub use list::{
    CURATE, FeedRef, ListBasic, ListDetails, ListInfo, ListItem, ListPage, ListView, ListViewer,
    MAX_PACK_FEEDS, MAX_PACK_PEOPLE, MODLIST, Membership, REFERENCE, StarterPack, StarterPackBasic,
    StarterPackRecord,
};
pub use live::{
    ActorStatus, LiveChat, LiveEvent, LiveNow, LiveStream, PastStream, StatusEmbed, StatusLink,
};
pub use notification::Notification;
pub use post::{ByteSlice, Facet, FacetFeature, PostRecord, PostView, PostViewer};
pub use prefs::{
    FeedSource, FollowingPrefs, LabelVisibility, MutedWord, PrefEdit, Preferences, SavedFeed,
};
pub use privacy::{DECLARATION as VISIBILITY_DECLARATION, Privacy, set_logged_out};
pub use translation::{TranslateService, Translation, base_lang, browser_url};
