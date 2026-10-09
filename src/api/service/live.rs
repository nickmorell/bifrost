//! The calls behind the Live screen: Streamplace for streams, Bluesky for the people
//! streaming and for sending chat.

use crate::api::streamplace::Node;
use crate::api::{ApiError, Client};
use crate::model::{AtUri, Author, Did, LiveStream, Page, PastStream, Profile};
use crate::net::shared;

/// A streamer's page: their profile and latest stream, which may have ended.
#[derive(Debug, Clone)]
pub struct Channel {
    pub profile: Profile,
    pub latest: Option<LiveStream>,
}

/// What the Live screen asks of Streamplace and Bluesky. Calls return futures that can
/// move to another thread, so they can run as iced tasks.
pub trait LiveApi: Clone + Send + Sync + 'static {
    /// The streams live now, and the profiles of the people streaming.
    fn live_now(
        &self,
    ) -> impl Future<Output = Result<(Vec<LiveStream>, Vec<Author>), ApiError>> + Send;

    /// `actor`'s profile and latest stream.
    fn channel(&self, actor: &str) -> impl Future<Output = Result<Channel, ApiError>> + Send;

    /// A page of `actor`'s past streams.
    fn past(
        &self,
        actor: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<PastStream>, ApiError>> + Send;

    /// Sends `text` to `streamer`'s chat.
    fn send(
        &self,
        streamer: &Did,
        text: &str,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
}

impl LiveApi for Client {
    fn live_now(
        &self,
    ) -> impl Future<Output = Result<(Vec<LiveStream>, Vec<Author>), ApiError>> + Send {
        let client = self.clone();
        Box::pin(async move {
            let streams = Node(shared()).live_now().await?;
            let dids: Vec<Did> = streams.iter().map(|s| s.did.clone()).collect();
            // Names and pictures are nice to have; the streams show without them.
            let authors = client.profiles(&dids).await.unwrap_or_default();
            Ok((streams, authors))
        })
    }

    fn channel(&self, actor: &str) -> impl Future<Output = Result<Channel, ApiError>> + Send {
        let client = self.clone();
        let actor = actor.to_string();
        Box::pin(async move {
            let profile = Box::pin(client.profile(&actor)).await?;
            let latest = Node(shared()).latest(profile.did.as_str()).await?;
            Ok(Channel { profile, latest })
        })
    }

    fn past(
        &self,
        actor: &str,
        cursor: Option<String>,
    ) -> impl Future<Output = Result<Page<PastStream>, ApiError>> + Send {
        let actor = actor.to_string();
        Box::pin(async move { Node(shared()).past(&actor, cursor.as_deref()).await })
    }

    fn send(
        &self,
        streamer: &Did,
        text: &str,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        let client = self.clone();
        let (streamer, text) = (streamer.clone(), text.to_string());
        Box::pin(async move { client.send_live_chat(&streamer, &text).await })
    }
}
