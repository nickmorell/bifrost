//! Your profile record: name, bio, pictures, links, the pinned post and self-labels,
//! each changed without losing the fields BiFrost does not edit.

use super::{AnyAgent, ApiError, Client, PROFILE, conv, now_iso, strong_ref, xrpc_err};
use crate::model::{AtUri, StrongRef, tid};
use serde_json::{Map, Value, json};

/// Each profile link is a record of its own; the profile lists them in order.
const LINK: &str = "app.bsky.actor.link";

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
    /// Your links, when you changed them.
    pub links: Option<LinksEdit>,
}

/// Your profile links as they will be saved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinksEdit {
    /// In the order the profile shows them.
    pub links: Vec<LinkEdit>,
    /// Links taken off the profile; their records are deleted.
    pub removed: Vec<AtUri>,
}

/// One profile link as it will be saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkEdit {
    /// The record it was loaded from; `None` for a new link.
    pub existing: Option<StrongRef>,
    /// An https address.
    pub url: String,
    /// Empty for none: apps then show the site's name.
    pub title: String,
    /// Whether the address or title changed since it was loaded.
    pub changed: bool,
}

impl Client {
    /// Reads your profile record, lets `change` edit it, and writes it back.
    pub(super) async fn edit_profile_record(
        &self,
        change: impl FnOnce(&mut Map<String, Value>),
    ) -> Result<(), ApiError> {
        let (collection, rkey) = PROFILE;
        let (mut record, swap) = match self.my_record(collection, rkey).await? {
            Some((Value::Object(map), cid)) => (map, Some(cid)),
            _ => (Map::new(), None),
        };
        record.insert("$type".into(), json!(collection));
        change(&mut record);
        self.put_record(collection, rkey, Value::Object(record), swap)
            .await?;
        Ok(())
    }

    /// Saves your name, bio, profile picture and banner. Fields BiFrost does not
    /// edit, such as the pinned post, are kept.
    pub async fn save_profile(&self, edit: ProfileEdit) -> Result<(), ApiError> {
        let avatar = match edit.avatar {
            ImageChange::Set(bytes) => Some(Some(self.upload(bytes).await?)),
            ImageChange::Remove => Some(None),
            ImageChange::Keep => None,
        };
        let banner = match edit.banner {
            ImageChange::Set(bytes) => Some(Some(self.upload(bytes).await?)),
            ImageChange::Remove => Some(None),
            ImageChange::Keep => None,
        };
        let links = match edit.links {
            Some(links) => Some(self.save_links(links).await?),
            None => None,
        };
        self.edit_profile_record(|r| {
            let mut text = |key: &str, value: &str| match value.trim() {
                "" => _ = r.remove(key),
                v => _ = r.insert(key.into(), json!(v)),
            };
            text("displayName", &edit.display_name);
            text("description", &edit.description);
            for (key, blob) in [("avatar", avatar), ("banner", banner)] {
                match blob {
                    Some(Some(b)) => _ = r.insert(key.into(), b),
                    Some(None) => _ = r.remove(key),
                    None => {}
                }
            }
            match links {
                Some(refs) if refs.is_empty() => _ = r.remove("links"),
                Some(refs) => _ = r.insert("links".into(), refs.iter().map(strong_ref).collect()),
                None => {}
            }
        })
        .await
    }

    /// Writes the link records that are new or changed and deletes the removed ones,
    /// all in one go, and returns every link's record in order for the profile to
    /// list.
    async fn save_links(&self, edit: LinksEdit) -> Result<Vec<StrongRef>, ApiError> {
        let mut writes = Vec::new();
        for link in edit
            .links
            .iter()
            .filter(|l| l.changed || l.existing.is_none())
        {
            writes.push(match &link.existing {
                Some(r) => {
                    let rkey = r.uri.rkey().unwrap_or_default().to_string();
                    let old = match self.my_record(LINK, &rkey).await? {
                        Some((Value::Object(map), _)) => map,
                        _ => Map::new(),
                    };
                    json!({
                        "$type": "com.atproto.repo.applyWrites#update",
                        "collection": LINK, "rkey": rkey, "value": link_record(old, link)
                    })
                }
                None => json!({
                    "$type": "com.atproto.repo.applyWrites#create",
                    "collection": LINK, "rkey": tid(), "value": link_record(Map::new(), link)
                }),
            });
        }
        for uri in &edit.removed {
            writes.push(json!({
                "$type": "com.atproto.repo.applyWrites#delete",
                "collection": LINK, "rkey": uri.rkey().unwrap_or_default()
            }));
        }
        let mut written = Vec::new();
        if !writes.is_empty() {
            let repo = self.my_did().await?;
            let out = xrpc!(self, |a| a
                .api
                .com
                .atproto
                .repo
                .apply_writes(conv(json!({"repo": repo, "writes": writes}))?));
            written = conv::<_, Vec<Value>>(&out.results)?
                .into_iter()
                .filter_map(|r| serde_json::from_value::<StrongRef>(r).ok())
                .collect();
        }
        // The results come in the order of the writes, so each new or changed link
        // takes the next one.
        let mut written = written.into_iter();
        edit.links
            .into_iter()
            .map(|l| match l.existing {
                Some(r) if !l.changed => Ok(r),
                _ => written
                    .next()
                    .ok_or_else(|| ApiError::Other("a link wasn't saved".into())),
            })
            .collect()
    }

    /// Pins a post to the top of your profile, or unpins whatever is pinned.
    pub async fn pin_post(&self, post: Option<StrongRef>) -> Result<(), ApiError> {
        self.edit_profile_record(|r| match &post {
            Some(p) => _ = r.insert("pinnedPost".into(), strong_ref(p)),
            None => _ = r.remove("pinnedPost"),
        })
        .await
    }
}

/// A link record with `link`'s address and title. A record that already existed keeps
/// its date and labels, and its icon unless the address changed.
fn link_record(mut record: Map<String, Value>, link: &LinkEdit) -> Value {
    if record.get("url").and_then(Value::as_str) != Some(link.url.as_str()) {
        record.remove("icon");
    }
    record.insert("$type".into(), json!(LINK));
    record.insert("url".into(), json!(link.url));
    match link.title.trim() {
        "" => _ = record.remove("title"),
        t => _ = record.insert("title".into(), json!(t)),
    }
    record
        .entry("createdAt")
        .or_insert_with(|| json!(now_iso()));
    Value::Object(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(url: &str, title: &str) -> LinkEdit {
        LinkEdit {
            existing: None,
            url: url.into(),
            title: title.into(),
            changed: true,
        }
    }

    #[test]
    fn a_new_link_record_has_its_address_title_and_date() {
        let r = link_record(Map::new(), &link("https://a.test", " Blog "));
        assert_eq!(r["$type"], LINK);
        assert_eq!(r["url"], "https://a.test");
        assert_eq!(r["title"], "Blog");
        assert!(r["createdAt"].is_string());
    }

    #[test]
    fn a_changed_link_keeps_what_still_fits() {
        let old = json!({"url": "https://a.test", "title": "Old", "icon": {"ref": "x"},
                         "createdAt": "2026-10-01T00:00:00Z", "labels": {"values": []}});
        let Value::Object(old) = old else {
            unreachable!()
        };

        let retitled = link_record(old.clone(), &link("https://a.test", ""));
        assert!(retitled.get("title").is_none());
        assert!(retitled.get("icon").is_some(), "same site, same icon");
        assert_eq!(retitled["createdAt"], "2026-10-01T00:00:00Z");
        assert!(retitled.get("labels").is_some());

        let moved = link_record(old, &link("https://b.test", "New"));
        assert!(
            moved.get("icon").is_none(),
            "the old site's icon would be wrong"
        );
        assert_eq!(moved["title"], "New");
    }
}
