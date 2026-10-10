//! Authenticated Bluesky API calls.
//!
//! [`Client`] wraps the active account from [`AccountManager`] and returns BiFrost's own
//! [`crate::model`] types. atrium types stay inside this file, just like in `auth::atrium`:
//! responses are converted through `serde_json`, so the UI never depends on them.

mod chat;
pub mod client;
mod error;
pub mod richtext;
mod server;
pub mod service;
pub mod streamplace;
mod web;
pub use error::ApiError;
use error::{answer, xrpc_err};
pub use server::{Credentials, DEFAULT_SERVICE, insecure_server};
pub use web::{fetch_bytes, github, link_card};

use crate::auth::atrium::{AtriumBackend, to_atp};
use crate::auth::oauth::{OAuth, OAuthAgent};
use crate::auth::xrpc::Xrpc;
use crate::auth::{AccountManager, Session};
use crate::model::{
    AtUri, Author, Did, FeedGenerator, FeedItem, FeedSource, Interaction, ListInfo, Notification,
    Page, PeopleKind, PostView, PrefEdit, Preferences, Profile, StrongRef, ThreadNode, Trend, tid,
};
use crate::storage::secrets::KeyringStore;
use atrium_api::agent::Configure;
use atrium_api::agent::atp_agent::AtpAgent;
use atrium_api::agent::atp_agent::store::MemorySessionStore;
use atrium_api::agent::bluesky::BSKY_LABELER_DID;
use atrium_api::xrpc::error::Error as XrpcError;
use atrium_common::store::Store;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::fmt::{Debug, Display};
use std::sync::Arc;

pub type Accounts = AccountManager<AtriumBackend, KeyringStore>;

const PAGE_SIZE: u8 = 40;
/// Posts fetched when checking a feed for new ones. More than this many new posts
/// shows as a gap, which "Show new" fills from the top.
const PEEK_SIZE: u8 = 10;

/// Converts between serde shapes: request JSON into atrium inputs, atrium outputs into
/// model types.
fn conv<T: Serialize, U: DeserializeOwned>(value: T) -> Result<U, ApiError> {
    let v = serde_json::to_value(value).map_err(|e| ApiError::Decode(e.to_string()))?;
    serde_json::from_value(v).map_err(|e| ApiError::Decode(e.to_string()))
}

/// Runs one XRPC call on the active account's agent and saves any tokens atrium
/// rotated while doing it.
macro_rules! xrpc {
    ($self:ident, |$agent:ident| $call:expr) => {{
        match $self.agent().await? {
            AnyAgent::Password(agent, before) => {
                let $agent = &agent;
                let result = $call.await;
                $self.save_rotated_tokens(&agent, &before).await;
                result.map_err(xrpc_err)?
            }
            AnyAgent::OAuth(agent) => {
                let $agent = &*agent;
                $call.await.map_err(xrpc_err)?
            }
        }
    }};
}

// After `xrpc!`, which they use.
mod articles;
mod lists;
mod live;
mod posting;
mod privacy;
mod profile_record;
mod video;
pub use posting::{DraftImage, PostDraft};
pub use video::{
    DraftVideo, MAX_VIDEO_BYTES, MAX_VIDEO_SECS, VideoInfo, is_video_name, probe as probe_video,
};

/// An OAuth agent and the account it was made for.
type CachedAgent = (String, Arc<OAuthAgent>);

/// The active account's connection: app-password tokens, or a browser sign-in.
type PasswordAgent = AtpAgent<MemorySessionStore, Xrpc>;
/// An app-password agent and the access token it was made with.
type CachedPasswordAgent = (String, Arc<PasswordAgent>);

enum AnyAgent {
    Password(Arc<PasswordAgent>, Session),
    OAuth(Arc<OAuthAgent>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkCard {
    pub url: String,
    pub title: String,
    pub description: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthorFilter {
    Posts,
    Replies,
    Media,
    /// Posts the actor liked. Bluesky only shows these for your own account.
    Likes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportReason {
    Spam,
    Violation,
    Misleading,
    Sexual,
    Rude,
    Other,
}

impl ReportReason {
    pub const ALL: [ReportReason; 6] = [
        Self::Spam,
        Self::Violation,
        Self::Misleading,
        Self::Sexual,
        Self::Rude,
        Self::Other,
    ];

    fn nsid(self) -> &'static str {
        match self {
            Self::Spam => "com.atproto.moderation.defs#reasonSpam",
            Self::Violation => "com.atproto.moderation.defs#reasonViolation",
            Self::Misleading => "com.atproto.moderation.defs#reasonMisleading",
            Self::Sexual => "com.atproto.moderation.defs#reasonSexual",
            Self::Rude => "com.atproto.moderation.defs#reasonRude",
            Self::Other => "com.atproto.moderation.defs#reasonOther",
        }
    }
}

impl Display for ReportReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Spam => "Spam",
            Self::Violation => "Breaks the rules",
            Self::Misleading => "Misleading",
            Self::Sexual => "Unwanted sexual content",
            Self::Rude => "Harassment or rudeness",
            Self::Other => "Something else",
        })
    }
}

/// What to do with a profile picture or banner when a profile is saved.
#[derive(Debug, Clone)]
pub enum ImageChange {
    Keep,
    Remove,
    /// Upload this image (already re-encoded for upload).
    Set(Vec<u8>),
}

/// The editable parts of your profile.
#[derive(Debug, Clone)]
pub struct ProfileEdit {
    pub display_name: String,
    pub description: String,
    pub avatar: ImageChange,
    pub banner: ImageChange,
}

const THREADGATE: &str = "app.bsky.feed.threadgate";
const POSTGATE: &str = "app.bsky.feed.postgate";

/// The collection and record key of your profile record.
const PROFILE: (&str, &str) = ("app.bsky.actor.profile", "self");

/// What is being reported.
#[derive(Debug, Clone)]
pub enum ReportSubject {
    Post(StrongRef),
    Account(Did),
}

#[derive(Clone)]
pub struct Client {
    accounts: Arc<Accounts>,
    oauth: Option<Arc<OAuth>>,
    /// The OAuth agent for the account it was made for, reused between calls.
    oauth_agent: Arc<tokio::sync::Mutex<Option<CachedAgent>>>,
    /// The app-password agent, reused while the access token it was made with is
    /// still the current one, so its connections are reused too.
    password_agent: Arc<std::sync::Mutex<Option<CachedPasswordAgent>>>,
}

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Client")
    }
}

impl Client {
    pub fn new(accounts: Arc<Accounts>) -> Self {
        let oauth = OAuth::new(Arc::new(KeyringStore::default()))
            .ok()
            .map(Arc::new);
        Self {
            accounts,
            oauth,
            oauth_agent: Arc::default(),
            password_agent: Arc::default(),
        }
    }

    fn oauth(&self) -> Result<&OAuth, ApiError> {
        self.oauth
            .as_deref()
            .ok_or_else(|| ApiError::Auth("signing in with the browser is unavailable".into()))
    }

    /// Signs in through the browser and makes that account active.
    pub async fn sign_in_with_browser(&self, handle: &str) -> Result<String, ApiError> {
        let signed_in = self.oauth()?.sign_in(handle).await?;
        self.accounts
            .add_oauth(&signed_in.did, &signed_in.handle, &signed_in.pds_url)?;
        *self.oauth_agent.lock().await = None;
        Ok(signed_in.did)
    }

    /// Logs an account out, revoking a browser sign-in with its server first.
    pub async fn logout(&self, did: &str) -> Result<(), ApiError> {
        if self.accounts.is_oauth(did)
            && let Ok(oauth) = self.oauth()
        {
            oauth.revoke(did).await;
        }
        *self.oauth_agent.lock().await = None;
        self.accounts.logout(Some(did)).await?;
        Ok(())
    }

    pub fn accounts(&self) -> &Accounts {
        &self.accounts
    }

    async fn agent(&self) -> Result<AnyAgent, ApiError> {
        let session = self.accounts.session().await?;
        if self.accounts.is_oauth(&session.did) {
            let mut cached = self.oauth_agent.lock().await;
            if let Some((did, agent)) = cached.as_ref()
                && *did == session.did
            {
                return Ok(AnyAgent::OAuth(agent.clone()));
            }
            let agent = Arc::new(self.oauth()?.agent(&session.did).await?);
            *cached = Some((session.did.clone(), agent.clone()));
            return Ok(AnyAgent::OAuth(agent));
        }
        if let Some((access, agent)) = self
            .password_agent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            && *access == session.access_jwt
        {
            return Ok(AnyAgent::Password(agent.clone(), session));
        }
        let store = MemorySessionStore::default();
        store
            .set((), to_atp(&session)?)
            .await
            .map_err(|e| ApiError::Other(e.to_string()))?;
        let agent = AtpAgent::new(Xrpc::new(&session.pds_url), store);
        // Redacting labels from Bluesky's own moderation service, as the official app
        // does. Subscribed third-party labelers come later.
        if let Ok(did) = BSKY_LABELER_DID.parse() {
            agent.configure_labelers_header(Some(vec![(did, true)]));
        }
        let agent = Arc::new(agent);
        *self
            .password_agent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some((session.access_jwt.clone(), agent.clone()));
        Ok(AnyAgent::Password(agent, session))
    }

    async fn save_rotated_tokens(&self, agent: &PasswordAgent, before: &Session) {
        let Some(now) = agent.get_session().await else {
            return;
        };
        if now.access_jwt != before.access_jwt || now.refresh_jwt != before.refresh_jwt {
            let updated = Session {
                access_jwt: now.access_jwt.clone(),
                refresh_jwt: now.refresh_jwt.clone(),
                ..before.clone()
            };
            // Losing a rotated refresh token would log the user out, but there is
            // nothing better to do with the error here than carry on.
            let _ = self.accounts.update_session(before, &updated).await;
        }
    }

    pub async fn my_did(&self) -> Result<Did, ApiError> {
        Ok(self.accounts.session().await?.did.into())
    }

    // ---- Reading -----------------------------------------------------------------

    pub async fn feed(
        &self,
        source: &FeedSource,
        cursor: Option<String>,
    ) -> Result<Page<FeedItem>, ApiError> {
        self.feed_page(source, cursor, PAGE_SIZE).await
    }

    /// The newest few posts of a feed, to check for new ones without fetching a page.
    pub async fn feed_peek(&self, source: &FeedSource) -> Result<Page<FeedItem>, ApiError> {
        self.feed_page(source, None, PEEK_SIZE).await
    }

    async fn feed_page(
        &self,
        source: &FeedSource,
        cursor: Option<String>,
        limit: u8,
    ) -> Result<Page<FeedItem>, ApiError> {
        let (items, cursor): (Vec<Value>, Option<String>) = match source {
            FeedSource::Following => {
                let out =
                    xrpc!(self, |a| a.api.app.bsky.feed.get_timeline(conv(
                        json!({"limit": limit, "cursor": cursor})
                    )?));
                (conv(&out.feed)?, out.data.cursor.clone())
            }
            FeedSource::Generator(uri) => {
                let out = xrpc!(self, |a| a.api.app.bsky.feed.get_feed(conv(
                    json!({"feed": uri, "limit": limit, "cursor": cursor})
                )?));
                (conv(&out.feed)?, out.data.cursor.clone())
            }
            FeedSource::List(uri) => {
                let out = xrpc!(self, |a| a.api.app.bsky.feed.get_list_feed(conv(
                    json!({"list": uri, "limit": limit, "cursor": cursor})
                )?));
                (conv(&out.feed)?, out.data.cursor.clone())
            }
        };
        Ok(Page {
            items: lenient_items(items),
            cursor,
        })
    }

    pub async fn author_feed(
        &self,
        actor: &str,
        filter: AuthorFilter,
        cursor: Option<String>,
    ) -> Result<Page<FeedItem>, ApiError> {
        let filter = match filter {
            AuthorFilter::Posts => "posts_and_author_threads",
            AuthorFilter::Replies => "posts_with_replies",
            AuthorFilter::Media => "posts_with_media",
            AuthorFilter::Likes => {
                let out = xrpc!(self, |a| a.api.app.bsky.feed.get_actor_likes(conv(
                    json!({"actor": actor, "limit": PAGE_SIZE, "cursor": cursor})
                )?));
                return Ok(Page {
                    items: lenient_items(conv(&out.feed)?),
                    cursor: out.data.cursor.clone(),
                });
            }
        };
        let out = xrpc!(self, |a| a.api.app.bsky.feed.get_author_feed(conv(
            json!({
                "actor": actor, "filter": filter, "limit": PAGE_SIZE, "cursor": cursor,
                "includePins": cursor.is_none()
            })
        )?));
        Ok(Page {
            items: lenient_items(conv(&out.feed)?),
            cursor: out.data.cursor.clone(),
        })
    }

    pub async fn thread(&self, uri: &str) -> Result<ThreadNode, ApiError> {
        let out = xrpc!(self, |a| a.api.app.bsky.feed.get_post_thread(conv(
            json!({"uri": uri, "depth": 6, "parentHeight": 30})
        )?));
        conv(&out.thread)
    }

    /// Posts by URI, 25 to a request (the API's limit), up to four requests at once.
    pub async fn posts(&self, uris: &[AtUri]) -> Result<Vec<PostView>, ApiError> {
        let mut all = Vec::new();
        for batch in uris.chunks(25 * 4) {
            let handles: Vec<_> = batch
                .chunks(25)
                .map(|chunk| {
                    let (client, chunk) = (self.clone(), chunk.to_vec());
                    tokio::spawn(async move { client.posts_chunk(&chunk).await })
                })
                .collect();
            // Awaited in order, so the posts keep the order they were asked for in.
            for h in handles {
                all.extend(h.await.map_err(|e| ApiError::Other(e.to_string()))??);
            }
        }
        Ok(all)
    }

    async fn posts_chunk(&self, chunk: &[AtUri]) -> Result<Vec<PostView>, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .feed
            .get_posts(conv(json!({ "uris": chunk }))?));
        Ok(lenient_items(conv(&out.posts)?))
    }

    pub async fn profile(&self, actor: &str) -> Result<Profile, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .actor
            .get_profile(conv(json!({ "actor": actor }))?));
        conv(&*out)
    }

    pub async fn notifications(
        &self,
        cursor: Option<String>,
    ) -> Result<Page<Notification>, ApiError> {
        let out = xrpc!(self, |a| a.api.app.bsky.notification.list_notifications(
            conv(json!({"limit": PAGE_SIZE, "cursor": cursor}))?
        ));
        Ok(Page {
            items: lenient_items(conv(&out.notifications)?),
            cursor: out.data.cursor.clone(),
        })
    }

    pub async fn unread_count(&self) -> Result<u64, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .notification
            .get_unread_count(conv(json!({}))?));
        Ok(out.count.max(0) as u64)
    }

    pub async fn mark_notifications_seen(&self) -> Result<(), ApiError> {
        let now = now_iso();
        xrpc!(self, |a| a
            .api
            .app
            .bsky
            .notification
            .update_seen(conv(json!({ "seenAt": now }))?));
        Ok(())
    }

    pub async fn preferences(&self) -> Result<Preferences, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .actor
            .get_preferences(conv(json!({}))?));
        let prefs: Vec<Value> = conv(&out.preferences)?;
        Ok(Preferences::from_json(&prefs))
    }

    /// Applies `edits` to the latest preferences on the server and saves them.
    pub async fn edit_preferences(&self, edits: Vec<PrefEdit>) -> Result<Preferences, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .actor
            .get_preferences(conv(json!({}))?));
        let mut prefs: Vec<Value> = conv(&out.preferences)?;
        for edit in &edits {
            edit.apply(&mut prefs);
        }
        xrpc!(self, |a| a
            .api
            .app
            .bsky
            .actor
            .put_preferences(conv(json!({ "preferences": prefs }))?));
        Ok(Preferences::from_json(&prefs))
    }

    /// Feeds Bluesky suggests, for the Discover section of the feeds screen.
    pub async fn suggested_feeds(&self) -> Result<Vec<FeedGenerator>, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .feed
            .get_suggested_feeds(conv(json!({ "limit": 30 }))?));
        Ok(lenient_items(conv(&out.feeds)?))
    }

    pub async fn feed_generators(&self, uris: &[AtUri]) -> Result<Vec<FeedGenerator>, ApiError> {
        if uris.is_empty() {
            return Ok(Vec::new());
        }
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .feed
            .get_feed_generators(conv(json!({ "feeds": uris }))?));
        Ok(lenient_items(conv(&out.feeds)?))
    }

    /// Saves a post to the account's bookmarks (private, synced with the official app).
    pub async fn bookmark(&self, subject: &StrongRef) -> Result<(), ApiError> {
        xrpc!(self, |a| a.api.app.bsky.bookmark.create_bookmark(conv(
            json!({"uri": subject.uri, "cid": subject.cid})
        )?));
        Ok(())
    }

    pub async fn unbookmark(&self, uri: &str) -> Result<(), ApiError> {
        xrpc!(self, |a| a
            .api
            .app
            .bsky
            .bookmark
            .delete_bookmark(conv(json!({ "uri": uri }))?));
        Ok(())
    }

    /// The account's bookmarked posts, newest first. Deleted or blocked ones are skipped.
    pub async fn bookmarks(&self, cursor: Option<String>) -> Result<Page<FeedItem>, ApiError> {
        let out =
            xrpc!(self, |a| a.api.app.bsky.bookmark.get_bookmarks(conv(
                json!({"limit": PAGE_SIZE, "cursor": cursor})
            )?));
        let items: Vec<Value> = conv(&out.bookmarks)?;
        let posts: Vec<Value> = items
            .into_iter()
            .filter_map(|b| b.get("item").cloned())
            .map(|post| json!({ "post": post }))
            .collect();
        Ok(Page {
            items: lenient_items(posts),
            cursor: out.data.cursor.clone(),
        })
    }

    /// Custom feeds the actor made.
    pub async fn actor_feeds(&self, actor: &str) -> Result<Vec<FeedGenerator>, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .feed
            .get_actor_feeds(conv(json!({"actor": actor, "limit": 100}))?));
        Ok(lenient_items(conv(&out.feeds)?))
    }

    /// Lists the actor made.
    pub async fn actor_lists(&self, actor: &str) -> Result<Vec<ListInfo>, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .graph
            .get_lists(conv(json!({"actor": actor, "limit": 100}))?));
        Ok(lenient_items(conv(&out.lists)?))
    }

    /// Who follows `actor`, or who `actor` follows.
    /// A page of accounts: someone's followers or follows, or your own muted or
    /// blocked accounts (`actor` is ignored for those).
    pub async fn people(
        &self,
        actor: &str,
        kind: PeopleKind,
        cursor: Option<String>,
    ) -> Result<Page<Author>, ApiError> {
        let params = json!({"actor": actor, "limit": PAGE_SIZE, "cursor": cursor});
        let (items, cursor): (Vec<Value>, Option<String>) = match kind {
            PeopleKind::Followers => {
                let out = xrpc!(self, |a| a.api.app.bsky.graph.get_followers(conv(params)?));
                (conv(&out.followers)?, out.data.cursor.clone())
            }
            PeopleKind::Following => {
                let out = xrpc!(self, |a| a.api.app.bsky.graph.get_follows(conv(params)?));
                (conv(&out.follows)?, out.data.cursor.clone())
            }
            PeopleKind::Muted => {
                let params = json!({"limit": PAGE_SIZE, "cursor": cursor});
                let out = xrpc!(self, |a| a.api.app.bsky.graph.get_mutes(conv(params)?));
                (conv(&out.mutes)?, out.data.cursor.clone())
            }
            PeopleKind::Blocked => {
                let params = json!({"limit": PAGE_SIZE, "cursor": cursor});
                let out = xrpc!(self, |a| a.api.app.bsky.graph.get_blocks(conv(params)?));
                (conv(&out.blocks)?, out.data.cursor.clone())
            }
        };
        Ok(Page {
            items: lenient_items(items),
            cursor,
        })
    }

    /// Display name of a list, for feed tabs.
    pub async fn list_name(&self, uri: &str) -> Result<String, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .graph
            .get_list(conv(json!({"list": uri, "limit": 1}))?));
        Ok(out.list.name.clone())
    }

    /// Searches posts, ordered by `sort`: "top" or "latest".
    pub async fn search_posts(
        &self,
        q: &str,
        sort: &str,
        cursor: Option<String>,
    ) -> Result<Page<PostView>, ApiError> {
        let out = xrpc!(self, |a| a.api.app.bsky.feed.search_posts(conv(
            json!({"q": q, "limit": PAGE_SIZE, "cursor": cursor, "sort": sort})
        )?));
        Ok(Page {
            items: lenient_items(conv(&out.posts)?),
            cursor: out.data.cursor.clone(),
        })
    }

    pub async fn search_actors(
        &self,
        q: &str,
        cursor: Option<String>,
    ) -> Result<Page<Author>, ApiError> {
        let out = xrpc!(self, |a| a.api.app.bsky.actor.search_actors(conv(
            json!({"q": q, "limit": PAGE_SIZE, "cursor": cursor})
        )?));
        Ok(Page {
            items: lenient_items(conv(&out.actors)?),
            cursor: out.data.cursor.clone(),
        })
    }

    /// Feeds whose name or description match `q`.
    pub async fn search_feeds(
        &self,
        q: &str,
        cursor: Option<String>,
    ) -> Result<Page<FeedGenerator>, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .unspecced
            .get_popular_feed_generators(conv(
                json!({"query": q, "limit": PAGE_SIZE, "cursor": cursor})
            )?));
        Ok(Page {
            items: lenient_items(conv(&out.feeds)?),
            cursor: out.data.cursor.clone(),
        })
    }

    /// What people are talking about right now, for the Explore page.
    pub async fn trending(&self) -> Result<Vec<Trend>, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .unspecced
            .get_trending_topics(conv(json!({ "limit": 14 }))?));
        Ok(lenient_items(conv(&out.topics)?))
    }

    /// Accounts Bluesky suggests following.
    pub async fn suggested_people(&self) -> Result<Vec<Author>, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .actor
            .get_suggestions(conv(json!({ "limit": 8 }))?));
        Ok(lenient_items(conv(&out.actors)?))
    }

    pub async fn typeahead(&self, q: &str) -> Result<Vec<Author>, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .actor
            .search_actors_typeahead(conv(json!({"q": q, "limit": 6}))?));
        Ok(lenient_items(conv(&out.actors)?))
    }

    pub async fn resolve_handle(&self, handle: &str) -> Result<Did, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .com
            .atproto
            .identity
            .resolve_handle(conv(json!({ "handle": handle }))?));
        Ok(out.did.to_string().into())
    }

    // ---- Writing -----------------------------------------------------------------

    /// One of your own records and its version, or `None` if it does not exist.
    async fn my_record(
        &self,
        collection: &str,
        rkey: &str,
    ) -> Result<Option<(Value, String)>, ApiError> {
        let repo = self.my_did().await?;
        let out = match self.agent().await? {
            AnyAgent::Password(agent, before) => {
                let r = agent
                    .api
                    .com
                    .atproto
                    .repo
                    .get_record(conv(
                        json!({"repo": repo, "collection": collection, "rkey": rkey}),
                    )?)
                    .await;
                self.save_rotated_tokens(&agent, &before).await;
                r
            }
            AnyAgent::OAuth(agent) => {
                agent
                    .api
                    .com
                    .atproto
                    .repo
                    .get_record(conv(
                        json!({"repo": repo, "collection": collection, "rkey": rkey}),
                    )?)
                    .await
            }
        };
        match out.map_err(xrpc_err) {
            Ok(out) => {
                let value: Value = conv(&out.value)?;
                let cid = out.cid.as_ref().map(conv::<_, String>).transpose()?;
                Ok(Some((value, cid.unwrap_or_default())))
            }
            Err(e) if e.is_not_found() => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Writes one of your own records. `swap` is the version it was read at, so a
    /// change made elsewhere in the meantime is not overwritten.
    async fn put_record(
        &self,
        collection: &str,
        rkey: &str,
        record: Value,
        swap: Option<String>,
    ) -> Result<String, ApiError> {
        let repo = self.my_did().await?;
        let mut input = json!({
            "repo": repo, "collection": collection, "rkey": rkey, "record": record
        });
        if let Some(cid) = swap.filter(|c| !c.is_empty()) {
            input["swapRecord"] = json!(cid);
        }
        let out = xrpc!(self, |a| a
            .api
            .com
            .atproto
            .repo
            .put_record(conv(input.clone())?));
        Ok(out.uri.clone())
    }

    async fn create_record(&self, collection: &str, record: Value) -> Result<AtUri, ApiError> {
        let repo = self.my_did().await?;
        let out = xrpc!(self, |a| a.api.com.atproto.repo.create_record(conv(
            json!({
                "repo": repo, "collection": collection, "record": record
            })
        )?));
        Ok(out.uri.clone().into())
    }

    /// Deletes the record at `uri` (a post, like, repost, follow or block).
    pub async fn delete(&self, uri: &str) -> Result<(), ApiError> {
        let (repo, collection, rkey) =
            parse_at_uri(uri).ok_or_else(|| ApiError::Other(format!("not a record URI: {uri}")))?;
        xrpc!(self, |a| a.api.com.atproto.repo.delete_record(conv(
            json!({
                "repo": repo, "collection": collection, "rkey": rkey
            })
        )?));
        Ok(())
    }

    pub async fn like(&self, subject: &StrongRef) -> Result<AtUri, ApiError> {
        self.create_record(
            "app.bsky.feed.like",
            json!({"$type": "app.bsky.feed.like", "subject": strong_ref(subject), "createdAt": now_iso()}),
        )
        .await
    }

    pub async fn repost(&self, subject: &StrongRef) -> Result<AtUri, ApiError> {
        self.create_record(
            "app.bsky.feed.repost",
            json!({"$type": "app.bsky.feed.repost", "subject": strong_ref(subject), "createdAt": now_iso()}),
        )
        .await
    }

    pub async fn follow(&self, did: &str) -> Result<AtUri, ApiError> {
        self.create_record(
            "app.bsky.graph.follow",
            json!({"$type": "app.bsky.graph.follow", "subject": did, "createdAt": now_iso()}),
        )
        .await
    }

    pub async fn block(&self, did: &str) -> Result<AtUri, ApiError> {
        self.create_record(
            "app.bsky.graph.block",
            json!({"$type": "app.bsky.graph.block", "subject": did, "createdAt": now_iso()}),
        )
        .await
    }

    pub async fn mute(&self, did: &str) -> Result<(), ApiError> {
        xrpc!(self, |a| a
            .api
            .app
            .bsky
            .graph
            .mute_actor(conv(json!({ "actor": did }))?));
        Ok(())
    }

    /// Stops notifications from the thread whose first post is `root`.
    pub async fn mute_thread(&self, root: &str) -> Result<(), ApiError> {
        xrpc!(self, |a| a
            .api
            .app
            .bsky
            .graph
            .mute_thread(conv(json!({ "root": root }))?));
        Ok(())
    }

    pub async fn unmute_thread(&self, root: &str) -> Result<(), ApiError> {
        xrpc!(self, |a| a
            .api
            .app
            .bsky
            .graph
            .unmute_thread(conv(json!({ "root": root }))?));
        Ok(())
    }

    pub async fn unmute(&self, did: &str) -> Result<(), ApiError> {
        xrpc!(self, |a| a
            .api
            .app
            .bsky
            .graph
            .unmute_actor(conv(json!({ "actor": did }))?));
        Ok(())
    }

    pub async fn report(
        &self,
        subject: &ReportSubject,
        reason: ReportReason,
        details: &str,
    ) -> Result<(), ApiError> {
        let subject = match subject {
            ReportSubject::Post(r) => {
                json!({"$type": "com.atproto.repo.strongRef", "uri": r.uri, "cid": r.cid})
            }
            ReportSubject::Account(did) => {
                json!({"$type": "com.atproto.admin.defs#repoRef", "did": did})
            }
        };
        let details = (!details.trim().is_empty()).then(|| details.trim().to_string());
        xrpc!(self, |a| a.api.com.atproto.moderation.create_report(conv(
            json!({
                "reasonType": reason.nsid(), "subject": subject, "reason": details
            })
        )?));
        Ok(())
    }

    async fn upload(&self, bytes: Vec<u8>) -> Result<Value, ApiError> {
        let out = xrpc!(self, |a| a.api.com.atproto.repo.upload_blob(bytes.clone()));
        conv(&out.blob)
    }

    /// Who can reply to and quote one of your posts.
    pub async fn post_interaction(&self, uri: &str) -> Result<Interaction, ApiError> {
        let (_, _, rkey) =
            parse_at_uri(uri).ok_or_else(|| ApiError::Other(format!("not a post: {uri}")))?;
        let gate = self.my_record(THREADGATE, &rkey).await?;
        let post = self.my_record(POSTGATE, &rkey).await?;
        Ok(Interaction::from_records(
            gate.as_ref().map(|g| &g.0),
            post.as_ref().map(|p| &p.0),
        ))
    }

    /// Changes who can reply to and quote one of your posts. `replies` only applies
    /// to the first post of a thread. Hidden replies and detached quotes are kept.
    pub async fn set_post_interaction(
        &self,
        uri: &str,
        interaction: Interaction,
        is_root: bool,
    ) -> Result<(), ApiError> {
        let (repo, _, rkey) =
            parse_at_uri(uri).ok_or_else(|| ApiError::Other(format!("not a post: {uri}")))?;
        if is_root {
            let existing = self.my_record(THREADGATE, &rkey).await?;
            let hidden = existing
                .as_ref()
                .and_then(|(v, _)| v.get("hiddenReplies"))
                .and_then(Value::as_array)
                .filter(|h| !h.is_empty())
                .cloned();
            match (&interaction.replies, hidden) {
                (None, None) => {
                    if existing.is_some() {
                        self.delete(&AtUri::new(&repo, THREADGATE, &rkey)).await?;
                    }
                }
                (rules, hidden) => {
                    let mut record =
                        json!({"$type": THREADGATE, "post": uri, "createdAt": now_iso()});
                    if let Some(r) = rules {
                        record["allow"] = Value::Array(r.to_json());
                    }
                    if let Some(h) = hidden {
                        record["hiddenReplies"] = Value::Array(h);
                    }
                    self.put_record(THREADGATE, &rkey, record, existing.map(|e| e.1))
                        .await?;
                }
            }
        }
        let existing = self.my_record(POSTGATE, &rkey).await?;
        let detached = existing
            .as_ref()
            .and_then(|(v, _)| v.get("detachedEmbeddingUris"))
            .and_then(Value::as_array)
            .filter(|d| !d.is_empty())
            .cloned();
        if interaction.quotes && detached.is_none() {
            if existing.is_some() {
                self.delete(&AtUri::new(&repo, POSTGATE, &rkey)).await?;
            }
            return Ok(());
        }
        let mut record = json!({
            "$type": POSTGATE, "post": uri, "createdAt": now_iso(),
            "embeddingRules": interaction.embedding_rules()
        });
        if let Some(d) = detached {
            record["detachedEmbeddingUris"] = Value::Array(d);
        }
        self.put_record(POSTGATE, &rkey, record, existing.map(|e| e.1))
            .await?;
        Ok(())
    }
}

/// Deserializes each element on its own and drops the ones that fail, so one odd
/// item never empties a page.
fn lenient_items<T: DeserializeOwned>(items: Vec<Value>) -> Vec<T> {
    items
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect()
}

fn strong_ref(r: &StrongRef) -> Value {
    json!({"uri": r.uri, "cid": r.cid})
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Splits `at://repo/collection/rkey`.
pub fn parse_at_uri(uri: &str) -> Option<(String, String, String)> {
    let rest = uri.strip_prefix("at://")?;
    let mut parts = rest.splitn(3, '/');
    let repo = parts.next()?.to_string();
    let collection = parts.next()?.to_string();
    let rkey = parts.next()?.to_string();
    (!rkey.is_empty() && !rkey.contains('/')).then_some((repo, collection, rkey))
}

#[cfg(test)]
mod tests;
