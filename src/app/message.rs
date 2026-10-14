//! The app's messages and the screens it can show (`Route`).

use super::{
    ProfileTab, account, actions, articles, atstore, chat, collections, compose, composer, deck,
    drafts, feeds, filters, interaction, lists, live, login, moderation, music, nav, notifications,
    people, pictures, popups, prefs, privacy, search, stats, thread, timeline, translate, tray,
    updates, video, viewer, window,
};
pub use crate::features::actions::Toggle;
use crate::model::{ArticleRef, AtUri, FeedSource, PeopleKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Home,
    Notifications,
    Search,
    Settings,
    Thread(AtUri),
    Profile(String, ProfileTab),
    /// Accounts related to the actor: followers, follows, or your muted or blocked
    /// accounts.
    People(String, PeopleKind),
    /// Content filters, muted words, and muted and blocked accounts.
    Moderation,
    /// Your saved and pinned feeds, and feeds to discover.
    Feeds,
    /// A custom feed or list opened from a profile.
    Feed(FeedSource, String),
    Bookmarks,
    Drafts,
    /// Your direct message conversations.
    Chats,
    /// One conversation, by id.
    Chat(String),
    /// One list, by uri.
    List(AtUri),
    /// One starter pack, by uri.
    StarterPack(AtUri),
    /// Your lists and starter packs.
    Lists,
    /// Who is streaming on Streamplace now.
    Live,
    /// A streamer's page, by handle or DID: their stream, chat and past streams.
    Channel(String),
    /// The ATstore directory of apps and services on the AT Protocol.
    Atstore,
    /// One app or service on ATstore, by its listing.
    Listing(AtUri),
    /// New articles from the publications you subscribe to.
    Articles,
    /// One article, in the reader.
    Article(ArticleRef),
    /// Your collections of posts.
    Collections,
    /// One collection, by id.
    Collection(u64),
    /// Posts kept on this computer, for reading while Bluesky can't be reached.
    Offline,
    /// What your posts of the last month did.
    Stats,
}

impl Route {
    pub(super) fn scroll_id(&self) -> String {
        format!("{self:?}")
    }
}

/// Where a link inside post text points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    Url(String),
    /// A link whose text names another site than the one it opens: `shown` is the
    /// address the text gives, `url` where it goes. Opening it asks first.
    Disguised {
        shown: String,
        url: String,
    },
    Profile(String),
    Tag(String),
}

/// The pop-up menus a post has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    /// Repost or quote.
    Repost,
    /// Everything else, under "…".
    More,
    /// The same two on a post shown above a reply it got, so that a post shown twice
    /// on screen (as itself and above its reply) opens one menu, not two.
    RepostAbove,
    MoreAbove,
    /// The same menu as "…", opened by right-clicking the post.
    Context,
    ContextAbove,
}

/// Everything the app reacts to. Most messages belong to one part of the app and are
/// grouped under it; the app passes each group on to that part.
#[derive(Debug, Clone)]
pub enum Message {
    /// The result of work started during session `0`. Results from a session that has
    /// since ended (an account switch or logout) are dropped, so one account's data can
    /// never land in another's view.
    Session(u64, Box<Message>),
    Account(account::Msg),
    Login(login::Msg),
    Prefs(prefs::Msg),
    Nav(nav::Msg),
    Window(window::Msg),
    Pictures(pictures::Msg),
    Viewer(viewer::Msg),
    Popup(popups::Msg),
    Action(actions::Msg),
    Compose(compose::Msg),
    Interaction(interaction::Msg),
    Tray(tray::Msg),
    Updates(updates::Msg),
    Alerts(crate::features::alerts::Msg),
    /// Time to check the unread counts.
    UnreadTick,
    /// The suggestion to try columns was taken (`true`) or dismissed.
    DeckHint(bool),
    DismissToast,
    /// A file BiFrost writes in the background could not be saved.
    SaveFailed(crate::storage::Failed),
    ToastExpired(u64),

    Timeline(timeline::Msg),
    Thread(thread::Msg),
    Profile(crate::features::profile::Msg),
    Notifications(notifications::Msg),
    Settings(crate::features::settings::Msg),
    Search(search::Msg),
    Chat(chat::Msg),
    Lists(lists::Msg),
    Live(live::Msg),
    Atstore(atstore::Msg),
    Deck(deck::Msg),
    Filter(filters::Msg),
    Translate(translate::Msg),
    Video(video::Msg),
    Composer(composer::Msg),
    People(people::Msg),
    Feeds(feeds::Msg),
    Moderation(moderation::Msg),
    Drafts(drafts::Msg),
    Articles(articles::Msg),
    Music(music::Msg),
    Collections(collections::Msg),
    Privacy(privacy::Msg),
    Stats(stats::Msg),
}
