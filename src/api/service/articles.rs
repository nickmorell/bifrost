//! The calls behind the article reader and your Standard.site subscriptions.

use crate::api::{ApiError, Client};
use crate::model::{Article, ArticleRef, ArticleSummary, AtUri};

/// Reading articles and following the publications they come from.
pub trait ArticlesApi: Clone + Send + Sync + 'static {
    fn article(
        &self,
        article: &ArticleRef,
    ) -> impl Future<Output = Result<Article, ApiError>> + Send;
    /// Your subscriptions: each record and the publication it follows.
    fn subscriptions(&self) -> impl Future<Output = Result<Vec<(AtUri, AtUri)>, ApiError>> + Send;
    /// Subscribes; gives the subscription's record.
    fn subscribe(
        &self,
        publication: &AtUri,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send;
    /// Removes a subscription record.
    fn unsubscribe(&self, record: &AtUri) -> impl Future<Output = Result<(), ApiError>> + Send;
    /// The newest articles from these publications, newest first.
    fn latest(&self, publications: &[AtUri]) -> impl Future<Output = Vec<ArticleSummary>> + Send;
}

impl ArticlesApi for Client {
    fn article(
        &self,
        article: &ArticleRef,
    ) -> impl Future<Output = Result<Article, ApiError>> + Send {
        Client::article(self, article)
    }
    fn subscriptions(&self) -> impl Future<Output = Result<Vec<(AtUri, AtUri)>, ApiError>> + Send {
        Client::article_subscriptions(self)
    }
    fn subscribe(
        &self,
        publication: &AtUri,
    ) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        Client::subscribe(self, publication)
    }
    fn unsubscribe(&self, record: &AtUri) -> impl Future<Output = Result<(), ApiError>> + Send {
        Client::delete(self, record)
    }
    fn latest(&self, publications: &[AtUri]) -> impl Future<Output = Vec<ArticleSummary>> + Send {
        Client::latest_articles(self, publications)
    }
}
