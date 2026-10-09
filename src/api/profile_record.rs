//! Your profile record: name, bio, pictures, the pinned post and self-labels, each
//! changed without losing the fields BiFrost does not edit.

use super::{ApiError, Client, ImageChange, PROFILE, ProfileEdit, strong_ref};
use crate::model::StrongRef;
use serde_json::{Value, json};

impl Client {
    /// Reads your profile record, lets `change` edit it, and writes it back.
    pub(super) async fn edit_profile_record(
        &self,
        change: impl FnOnce(&mut serde_json::Map<String, Value>),
    ) -> Result<(), ApiError> {
        let (collection, rkey) = PROFILE;
        let (mut record, swap) = match self.my_record(collection, rkey).await? {
            Some((Value::Object(map), cid)) => (map, Some(cid)),
            _ => (serde_json::Map::new(), None),
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
        })
        .await
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
