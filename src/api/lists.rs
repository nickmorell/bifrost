//! Lists (curated lists and moderation lists) and starter packs: reading them,
//! making and editing your own, and following or muting them.

use super::{
    AnyAgent, ApiError, Client, PAGE_SIZE, conv, lenient_items, now_iso, parse_at_uri, tid,
    xrpc_err,
};
use crate::model::{
    AtUri, CURATE, Did, ListDetails, ListItem, ListPage, ListView, MAX_PACK_FEEDS, MAX_PACK_PEOPLE,
    MODLIST, Membership, Page, REFERENCE, StarterPack, StarterPackBasic,
};
use serde_json::{Value, json};

const LIST: &str = "app.bsky.graph.list";
const LIST_ITEM: &str = "app.bsky.graph.listitem";
const LIST_BLOCK: &str = "app.bsky.graph.listblock";
const STARTER_PACK: &str = "app.bsky.graph.starterpack";

fn bad_uri(uri: &str) -> ApiError {
    ApiError::Other(format!("not a record: {uri}"))
}

fn text_or_none(s: &str) -> Value {
    match s.trim() {
        "" => Value::Null,
        t => json!(t),
    }
}

impl Client {
    /// A list's details and a page of the people on it.
    pub async fn list(&self, uri: &str, cursor: Option<String>) -> Result<ListPage, ApiError> {
        let out = xrpc!(self, |a| a.api.app.bsky.graph.get_list(conv(
            json!({"list": uri, "limit": PAGE_SIZE, "cursor": cursor})
        )?));
        let v: Value = conv(&out)?;
        Ok(ListPage {
            list: serde_json::from_value(v["list"].clone())
                .map_err(|e| ApiError::Decode(e.to_string()))?,
            items: Page {
                items: lenient_items(
                    serde_json::from_value(v["items"].clone()).unwrap_or_default(),
                ),
                cursor: v["cursor"].as_str().map(str::to_string),
            },
        })
    }

    /// Everyone on a list, following its pages up to `max` people.
    pub async fn list_members(&self, uri: &str, max: usize) -> Result<Vec<ListItem>, ApiError> {
        let mut all = Vec::new();
        let mut cursor = None;
        loop {
            let page = self.list(uri, cursor).await?;
            all.extend(page.items.items);
            cursor = page.items.cursor;
            if cursor.is_none() || all.len() >= max {
                return Ok(all);
            }
        }
    }

    /// Your lists, each saying whether `actor` is on it. Starter pack lists are
    /// left out; they are edited through their pack.
    pub async fn my_lists_for(&self, actor: &str) -> Result<Vec<Membership>, ApiError> {
        let out =
            xrpc!(self, |a| a.api.app.bsky.graph.get_lists_with_membership(
                conv(json!({"actor": actor, "limit": 100}))?
            ));
        let v: Value = conv(&out)?;
        let rows = v["listsWithMembership"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let list: ListView = serde_json::from_value(r["list"].clone()).ok()?;
                let item = r["listItem"]["uri"].as_str().map(AtUri::from);
                (list.purpose != REFERENCE).then_some(Membership { list, item })
            })
            .collect())
    }

    /// Makes a list and returns its URI.
    pub async fn create_list(&self, details: &ListDetails) -> Result<AtUri, ApiError> {
        let purpose = if details.purpose == MODLIST {
            MODLIST
        } else {
            CURATE
        };
        self.create_record(
            LIST,
            json!({
                "$type": LIST, "purpose": purpose, "name": details.name.trim(),
                "description": text_or_none(&details.description), "createdAt": now_iso()
            }),
        )
        .await
    }

    /// Renames a list or changes its description, keeping everything else.
    pub async fn edit_list(&self, uri: &str, details: &ListDetails) -> Result<(), ApiError> {
        let (_, _, rkey) = parse_at_uri(uri).ok_or_else(|| bad_uri(uri))?;
        let Some((Value::Object(mut record), cid)) = self.my_record(LIST, &rkey).await? else {
            return Err(ApiError::Other("that list no longer exists".into()));
        };
        record.insert("name".into(), json!(details.name.trim()));
        match text_or_none(&details.description) {
            Value::Null => _ = record.remove("description"),
            d => _ = record.insert("description".into(), d),
        }
        record.remove("descriptionFacets");
        self.put_record(LIST, &rkey, Value::Object(record), Some(cid))
            .await?;
        Ok(())
    }

    /// Deletes a list and the records that put people on it.
    pub async fn delete_list(&self, uri: &str) -> Result<(), ApiError> {
        let items = self.list_members(uri, 5000).await.unwrap_or_default();
        let deletes: Vec<Value> = items
            .iter()
            .filter_map(|i| delete_write(&i.uri))
            .chain(delete_write(uri))
            .collect();
        self.apply_writes(deletes).await
    }

    /// Puts someone on one of your lists and returns the record that does it.
    pub async fn add_to_list(&self, list: &str, did: &str) -> Result<AtUri, ApiError> {
        self.create_record(
            LIST_ITEM,
            json!({"$type": LIST_ITEM, "subject": did, "list": list, "createdAt": now_iso()}),
        )
        .await
    }

    /// Mutes or unmutes everyone on a moderation list.
    pub async fn mute_list(&self, uri: &str, mute: bool) -> Result<(), ApiError> {
        let params = json!({"list": uri});
        if mute {
            xrpc!(self, |a| a
                .api
                .app
                .bsky
                .graph
                .mute_actor_list(conv(params.clone())?));
        } else {
            xrpc!(self, |a| a
                .api
                .app
                .bsky
                .graph
                .unmute_actor_list(conv(params.clone())?));
        }
        Ok(())
    }

    /// Blocks everyone on a moderation list, returning the record that does it.
    pub async fn block_list(&self, uri: &str) -> Result<AtUri, ApiError> {
        self.create_record(
            LIST_BLOCK,
            json!({"$type": LIST_BLOCK, "subject": uri, "createdAt": now_iso()}),
        )
        .await
    }

    pub async fn starter_pack(&self, uri: &str) -> Result<StarterPack, ApiError> {
        let out = xrpc!(self, |a| a
            .api
            .app
            .bsky
            .graph
            .get_starter_pack(conv(json!({"starterPack": uri}))?));
        let v: Value = conv(&out)?;
        serde_json::from_value(v["starterPack"].clone())
            .map_err(|e| ApiError::Decode(e.to_string()))
    }

    pub async fn actor_starter_packs(
        &self,
        actor: &str,
    ) -> Result<Vec<StarterPackBasic>, ApiError> {
        let out =
            xrpc!(self, |a| a.api.app.bsky.graph.get_actor_starter_packs(
                conv(json!({"actor": actor, "limit": 100}))?
            ));
        let v: Value = conv(&out)?;
        Ok(lenient_items(
            serde_json::from_value(v["starterPacks"].clone()).unwrap_or_default(),
        ))
    }

    /// Makes a starter pack, with the list that holds its people, and returns the
    /// pack's URI. You are put in it first, as the official app does.
    pub async fn create_starter_pack(
        &self,
        details: &ListDetails,
        people: &[Did],
    ) -> Result<AtUri, ApiError> {
        let repo = self.my_did().await?;
        let list_rkey = tid();
        let pack_rkey = tid();
        let list_uri = AtUri::new(&repo, LIST, &list_rkey);
        let now = now_iso();
        let mut writes = vec![create_write(
            LIST,
            &list_rkey,
            json!({
                "$type": LIST, "purpose": REFERENCE, "name": details.name.trim(),
                "description": text_or_none(&details.description), "createdAt": now
            }),
        )];
        let mut members: Vec<&str> = vec![repo.as_str()];
        members.extend(people.iter().map(Did::as_str).filter(|d| *d != repo));
        for did in members.into_iter().take(MAX_PACK_PEOPLE) {
            writes.push(create_write(
                LIST_ITEM,
                &tid(),
                json!({"$type": LIST_ITEM, "subject": did, "list": list_uri, "createdAt": now}),
            ));
        }
        writes.push(create_write(
            STARTER_PACK,
            &pack_rkey,
            json!({
                "$type": STARTER_PACK, "name": details.name.trim(),
                "description": text_or_none(&details.description),
                "list": list_uri, "feeds": [], "createdAt": now
            }),
        ));
        self.apply_writes(writes).await?;
        Ok(AtUri::new(&repo, STARTER_PACK, &pack_rkey))
    }

    /// Renames a starter pack (and its list) or changes its description or feeds.
    pub async fn edit_starter_pack(
        &self,
        uri: &str,
        details: &ListDetails,
        feeds: &[AtUri],
    ) -> Result<(), ApiError> {
        let (_, _, rkey) = parse_at_uri(uri).ok_or_else(|| bad_uri(uri))?;
        let Some((Value::Object(mut record), cid)) = self.my_record(STARTER_PACK, &rkey).await?
        else {
            return Err(ApiError::Other("that starter pack no longer exists".into()));
        };
        record.insert("name".into(), json!(details.name.trim()));
        match text_or_none(&details.description) {
            Value::Null => _ = record.remove("description"),
            d => _ = record.insert("description".into(), d),
        }
        record.remove("descriptionFacets");
        let feeds: Vec<Value> = feeds
            .iter()
            .take(MAX_PACK_FEEDS)
            .map(|f| json!({"uri": f}))
            .collect();
        record.insert("feeds".into(), Value::Array(feeds));
        record.insert("updatedAt".into(), json!(now_iso()));
        let list = record
            .get("list")
            .and_then(Value::as_str)
            .map(str::to_string);
        self.put_record(STARTER_PACK, &rkey, Value::Object(record), Some(cid))
            .await?;
        if let Some(list) = list {
            // The list keeps the pack's name, as the official app does.
            self.edit_list(&list, details).await?;
        }
        Ok(())
    }

    /// Deletes a starter pack, its list and the list's entries.
    pub async fn delete_starter_pack(&self, uri: &str, list: Option<&str>) -> Result<(), ApiError> {
        if let Some(list) = list {
            self.delete_list(list).await?;
        }
        self.delete(uri).await
    }

    /// Follows everyone in `dids`, in batches, returning (did, follow record) for each.
    pub async fn follow_all(&self, dids: &[Did]) -> Result<Vec<(Did, AtUri)>, ApiError> {
        let repo = self.my_did().await?;
        let mut done = Vec::new();
        for chunk in dids.chunks(50) {
            let now = now_iso();
            let mut writes = Vec::new();
            let mut made = Vec::new();
            for did in chunk.iter().filter(|d| **d != repo) {
                let rkey = tid();
                made.push((
                    did.clone(),
                    AtUri::new(&repo, "app.bsky.graph.follow", &rkey),
                ));
                writes.push(create_write(
                    "app.bsky.graph.follow",
                    &rkey,
                    json!({"$type": "app.bsky.graph.follow", "subject": did, "createdAt": now}),
                ));
            }
            if !writes.is_empty() {
                self.apply_writes(writes).await?;
                done.extend(made);
            }
        }
        Ok(done)
    }

    /// Several record changes at once, at most 200 per call.
    async fn apply_writes(&self, writes: Vec<Value>) -> Result<(), ApiError> {
        let repo = self.my_did().await?;
        for chunk in writes.chunks(200) {
            xrpc!(self, |a| a
                .api
                .com
                .atproto
                .repo
                .apply_writes(conv(json!({"repo": repo, "writes": chunk}))?));
        }
        Ok(())
    }
}

fn create_write(collection: &str, rkey: &str, value: Value) -> Value {
    json!({
        "$type": "com.atproto.repo.applyWrites#create",
        "collection": collection, "rkey": rkey, "value": value
    })
}

fn delete_write(uri: &str) -> Option<Value> {
    let (_, collection, rkey) = parse_at_uri(uri)?;
    Some(json!({
        "$type": "com.atproto.repo.applyWrites#delete",
        "collection": collection, "rkey": rkey
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_lists_and_packs() {
        let list: ListView = serde_json::from_value(json!({
            "uri": "at://did:plc:a/app.bsky.graph.list/1", "cid": "c", "name": "Rustaceans",
            "purpose": MODLIST, "creator": {"did": "did:plc:a", "handle": "a.test"},
            "listItemCount": 4, "viewer": {"muted": true}, "indexedAt": "x"
        }))
        .unwrap();
        assert!(list.is_mod() && list.viewer.muted);
        assert_eq!(list.kind_label(), "Moderation list");
        let pack: StarterPack = serde_json::from_value(json!({
            "uri": "at://did:plc:a/app.bsky.graph.starterpack/1", "cid": "c",
            "record": {"$type": STARTER_PACK, "name": "Rust folks", "list": "at://l",
                "feeds": [{"uri": "at://f"}], "createdAt": "x"},
            "creator": {"did": "did:plc:a", "handle": "a.test"},
            "list": {"uri": "at://l", "name": "Rust folks", "purpose": REFERENCE, "listItemCount": 12},
            "feeds": [{"uri": "at://f", "displayName": "Rust", "creator": {"did": "d", "handle": "h"}}],
            "joinedAllTimeCount": 30
        }))
        .unwrap();
        assert_eq!(pack.record.name, "Rust folks");
        assert_eq!(pack.list.unwrap().list_item_count, Some(12));
        assert_eq!(pack.feeds.len(), 1);
        assert_eq!(
            delete_write("at://did:plc:a/app.bsky.graph.listitem/3").unwrap()["rkey"],
            "3"
        );
    }
}
