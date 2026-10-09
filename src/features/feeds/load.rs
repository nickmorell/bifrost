//! Looking up feed and list names, which takes a call per kind of feed.

use crate::api::service::FeedsApi;
use crate::model::{AtUri, FeedSource};

fn generators(sources: &[FeedSource]) -> Vec<AtUri> {
    sources
        .iter()
        .filter_map(|s| match s {
            FeedSource::Generator(u) => Some(u.clone()),
            _ => None,
        })
        .collect()
}

/// Names for Home tabs; `None` for a feed or list that no longer exists.
pub(super) async fn names(
    api: &impl FeedsApi,
    sources: Vec<FeedSource>,
) -> Vec<(FeedSource, Option<String>)> {
    let gens = generators(&sources);
    let mut out = Vec::new();
    if let Ok(found) = api.feed_generators(&gens).await {
        // The server leaves out feeds that no longer exist.
        for uri in gens {
            let name = found
                .iter()
                .find(|g| g.uri == uri)
                .map(|g| g.display_name.clone());
            out.push((FeedSource::Generator(uri), name));
        }
    }
    for s in sources {
        if let FeedSource::List(uri) = &s {
            match api.list_name(uri).await {
                Ok(name) => out.push((s.clone(), Some(name))),
                Err(e) if e.is_not_found() => out.push((s.clone(), None)),
                Err(_) => out.push((s.clone(), Some("List".into()))),
            }
        }
    }
    out
}

/// Names and pictures of saved feeds, for the Feeds screen. Ones that fail are left
/// out.
pub(super) async fn info(
    api: &impl FeedsApi,
    sources: Vec<FeedSource>,
) -> Vec<(FeedSource, String, Option<String>)> {
    let mut out = Vec::new();
    if let Ok(found) = api.feed_generators(&generators(&sources)).await {
        for g in found {
            out.push((FeedSource::Generator(g.uri), g.display_name, g.avatar));
        }
    }
    for s in sources {
        if let FeedSource::List(uri) = &s
            && let Ok(name) = api.list_name(uri).await
        {
            out.push((s.clone(), name, None));
        }
    }
    out
}
