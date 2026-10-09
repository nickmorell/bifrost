//! Unsent posts kept on this computer, so a post is never lost by navigating away.
//!
//! Drafts live in `config_dir/bifrost/drafts/`: `drafts.json` holds the text and
//! settings, and each attached image is a file beside it. Drafts belong to the account
//! that wrote them.

use crate::model::AtUri;
pub use crate::model::{Draft, DraftImageFile};
use crate::platform::paths;
use std::path::PathBuf;

/// What the composer hands over to be saved.
#[derive(Debug, Clone, Default)]
pub struct NewDraft {
    pub text: String,
    pub reply_to: Option<AtUri>,
    pub quote: Option<AtUri>,
    pub lang: String,
    /// Image bytes, alt text, width, height.
    pub images: Vec<(Vec<u8>, String, u32, u32)>,
    /// When to send it (RFC 3339), for a scheduled post.
    pub send_at: Option<String>,
}

pub struct DraftStore {
    dir: Option<PathBuf>,
}

impl Default for DraftStore {
    fn default() -> Self {
        Self {
            dir: paths::config_dir().map(|d| d.join("drafts")),
        }
    }
}

impl DraftStore {
    pub fn at(dir: PathBuf) -> Self {
        Self { dir: Some(dir) }
    }

    fn index(&self) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join("drafts.json"))
    }

    fn read_all(&self) -> Vec<Draft> {
        self.index()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn write_all(&self, drafts: &[Draft]) -> std::io::Result<()> {
        let (Some(dir), Some(index)) = (&self.dir, self.index()) else {
            return Ok(());
        };
        paths::create_dir(dir)?;
        let json = serde_json::to_vec_pretty(drafts).map_err(std::io::Error::other)?;
        // Write then rename, so a crash never leaves a half-written index.
        let tmp = dir.join("drafts.json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, index)
    }

    /// The account's drafts, newest first.
    pub fn list(&self, account: &str) -> Vec<Draft> {
        let mut out: Vec<Draft> = self
            .read_all()
            .into_iter()
            .filter(|d| d.account == account)
            .collect();
        out.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
        out
    }

    pub fn save(&self, account: &str, new: NewDraft) -> std::io::Result<Draft> {
        let Some(dir) = &self.dir else {
            return Err(std::io::Error::other("no config folder"));
        };
        paths::create_dir(dir)?;
        let now = chrono::Utc::now();
        let mut all = self.read_all();
        // Unique even when two drafts are saved within the same millisecond.
        let mut id = now.timestamp_millis().max(0) as u64;
        while all.iter().any(|d| d.id == id) {
            id += 1;
        }
        let mut images = Vec::new();
        for (i, (bytes, alt, width, height)) in new.images.into_iter().enumerate() {
            let file = format!("{id}-{i}.img");
            std::fs::write(dir.join(&file), bytes)?;
            images.push(DraftImageFile {
                file,
                alt,
                width,
                height,
            });
        }
        let draft = Draft {
            id,
            account: account.to_string(),
            saved_at: now.to_rfc3339(),
            text: new.text,
            reply_to: new.reply_to,
            quote: new.quote,
            lang: new.lang,
            images,
            send_at: new.send_at,
        };
        all.push(draft.clone());
        self.write_all(&all)?;
        Ok(draft)
    }

    /// Schedules a draft to be sent at `send_at` (RFC 3339), or unschedules it.
    pub fn schedule(&self, id: u64, send_at: Option<String>) -> std::io::Result<()> {
        let mut all = self.read_all();
        if let Some(d) = all.iter_mut().find(|d| d.id == id) {
            d.send_at = send_at;
        }
        self.write_all(&all)
    }

    /// Where an image file named in `drafts.json` lives. Only bare names count, so an
    /// edited `drafts.json` can't point at files elsewhere.
    fn image_path(&self, file: &str) -> Option<PathBuf> {
        let ok =
            !file.is_empty() && file != "." && file != ".." && !file.contains(['/', '\\', ':']);
        ok.then(|| self.dir.as_ref().map(|d| d.join(file)))
            .flatten()
    }

    /// Deletes every draft of one account, when it logs out.
    pub fn delete_account(&self, account: &str) -> std::io::Result<()> {
        let mut all = self.read_all();
        for d in all.iter().filter(|d| d.account == account) {
            for i in &d.images {
                if let Some(p) = self.image_path(&i.file) {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        all.retain(|d| d.account != account);
        self.write_all(&all)
    }

    /// Deletes the drafts folder, for "Remove all BiFrost data".
    pub fn delete_everything(&self) {
        if let Some(dir) = &self.dir {
            // Its folder goes too, last, and that failure is the one reported.
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    /// Reads a draft's image files: bytes, alt text, width, height.
    pub fn images(&self, draft: &Draft) -> Vec<(Vec<u8>, String, u32, u32)> {
        draft
            .images
            .iter()
            .filter_map(|i| {
                let bytes = std::fs::read(self.image_path(&i.file)?).ok()?;
                Some((bytes, i.alt.clone(), i.width, i.height))
            })
            .collect()
    }

    pub fn delete(&self, id: u64) -> std::io::Result<()> {
        let mut all = self.read_all();
        if let Some(d) = all.iter().find(|d| d.id == id) {
            for i in &d.images {
                if let Some(p) = self.image_path(&i.file) {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        all.retain(|d| d.id != id);
        self.write_all(&all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drafts_round_trip_per_account() {
        let dir = std::env::temp_dir().join(format!("bifrost-drafts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = DraftStore::at(dir.clone());
        let a = store
            .save(
                "did:a",
                NewDraft {
                    text: "hello".into(),
                    reply_to: Some("at://x/app.bsky.feed.post/1".into()),
                    images: vec![(vec![1, 2, 3], "alt".into(), 4, 3)],
                    ..Default::default()
                },
            )
            .unwrap();
        store
            .save(
                "did:b",
                NewDraft {
                    text: "other account".into(),
                    ..Default::default()
                },
            )
            .unwrap();

        let mine = store.list("did:a");
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].text, "hello");
        assert_eq!(
            store.images(&mine[0]),
            vec![(vec![1, 2, 3], "alt".into(), 4, 3)]
        );

        store
            .schedule(a.id, Some("2026-10-09T09:00:00Z".into()))
            .unwrap();
        assert_eq!(
            store.list("did:a")[0].send_at.as_deref(),
            Some("2026-10-09T09:00:00Z")
        );
        store.delete(a.id).unwrap();
        assert!(store.list("did:a").is_empty());
        assert_eq!(store.list("did:b").len(), 1);
        assert!(!dir.join(format!("{}-0.img", a.id)).exists());
        store.delete_account("did:b").unwrap();
        assert!(store.list("did:b").is_empty());
        assert!(store.image_path("../secret").is_none());
        assert!(store.image_path("C:\\x").is_none());
        assert!(store.image_path("1-0.img").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
