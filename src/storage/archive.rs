//! Posts kept on this computer for each account: everything loaded in your
//! timelines, threads and bookmarks, up to [`MAX_KEPT`], so they can be searched and
//! read again without the network.

use super::later::Later;
use crate::model::{Did, Kept, Query};
use crate::platform::paths;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The most posts kept per account. The oldest seen go first; liked, saved and
/// your own posts are kept longest.
pub const MAX_KEPT: usize = 20_000;

/// How often kept posts are written out, except when BiFrost quits.
const SAVE_EVERY: Duration = Duration::from_mins(5);

/// Most results a search gives.
const MAX_FOUND: usize = 200;

#[derive(Debug, Default)]
pub struct Archive {
    account: Did,
    posts: Vec<Kept>,
    /// Where each post is in `posts`, by its URI.
    at: HashMap<String, usize>,
    /// Changed since it was last saved.
    dirty: bool,
    /// When it was last saved, so a large file is not written every few seconds.
    saved: Option<Instant>,
}

impl Archive {
    fn path(account: &str) -> Option<PathBuf> {
        let name: String = account
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        Some(
            paths::config_dir()?
                .join("archive")
                .join(format!("{name}.json")),
        )
    }

    /// The posts kept for `account`, or none if there are none or they can't be read.
    pub fn load(account: &Did) -> Self {
        let posts = Self::path(account)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self::with(account.clone(), posts)
    }

    fn with(account: Did, posts: Vec<Kept>) -> Self {
        let mut a = Self {
            account,
            posts,
            ..Self::default()
        };
        a.reindex();
        a
    }

    fn reindex(&mut self) {
        self.at = self
            .posts
            .iter()
            .enumerate()
            .map(|(i, k)| (k.uri.to_string(), i))
            .collect();
    }

    pub fn account(&self) -> &Did {
        &self.account
    }

    /// Takes in posts just loaded: new ones are added, known ones updated.
    pub fn keep(&mut self, posts: impl IntoIterator<Item = Kept>) {
        for k in posts {
            if let Some(&i) = self.at.get(k.uri.as_str()) {
                let mut same = k.clone();
                same.seen.clone_from(&self.posts[i].seen);
                if same != self.posts[i] {
                    self.posts[i].refresh(k);
                    self.dirty = true;
                }
            } else {
                self.at.insert(k.uri.to_string(), self.posts.len());
                self.posts.push(k);
                self.dirty = true;
            }
        }
        if self.posts.len() > MAX_KEPT {
            self.trim();
        }
    }

    /// Drops the oldest seen posts that are not liked, saved or yours.
    fn trim(&mut self) {
        let mut order: Vec<usize> = (0..self.posts.len()).collect();
        let precious = |k: &Kept| k.liked || k.saved || k.mine;
        order.sort_by(|&a, &b| {
            let (a, b) = (&self.posts[a], &self.posts[b]);
            precious(b)
                .cmp(&precious(a))
                .then_with(|| b.seen.cmp(&a.seen))
        });
        order.truncate(MAX_KEPT);
        order.sort_unstable();
        let mut keep = order.into_iter().peekable();
        let mut i = 0;
        self.posts.retain(|_| {
            let kept = keep.peek() == Some(&i);
            if kept {
                keep.next();
            }
            i += 1;
            kept
        });
        self.reindex();
    }

    /// Posts matching `q`, newest first.
    pub fn search(&self, q: &Query) -> Vec<&Kept> {
        let mut found: Vec<&Kept> = self.posts.iter().filter(|k| q.matches(k)).collect();
        found.sort_by(|a, b| b.created.cmp(&a.created));
        found.truncate(MAX_FOUND);
        found
    }

    /// Every kept post, in the order they were first kept.
    pub fn posts(&self) -> &[Kept] {
        &self.posts
    }

    pub fn len(&self) -> usize {
        self.posts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.posts.is_empty()
    }

    /// Saves the posts if they changed, in the background at most every few minutes,
    /// or before returning when BiFrost is about to quit (`now`).
    pub fn save(&mut self, now: bool) {
        static FILE: Later = Later::new("the posts kept for searching");
        let recently = self.saved.is_some_and(|t| t.elapsed() < SAVE_EVERY);
        if !self.dirty || self.account.is_empty() || (recently && !now) {
            return;
        }
        self.saved = Some(Instant::now());
        if let (Some(path), Ok(json)) = (Self::path(&self.account), serde_json::to_vec(&self.posts))
        {
            if now {
                FILE.write_now(&path, &json);
            } else {
                FILE.write(path, json);
            }
            self.dirty = false;
        }
    }

    /// Deletes what is kept for `account`, when it logs out.
    pub fn forget(account: &str) {
        if let Some(p) = Self::path(account) {
            // Nothing kept is nothing to delete.
            let _ = std::fs::remove_file(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AtUri;

    fn kept(n: usize, seen: &str) -> Kept {
        Kept {
            uri: AtUri::from(format!("at://did:plc:a/app.bsky.feed.post/{n}")),
            did: Did::from("did:plc:a"),
            handle: "a.test".into(),
            name: String::new(),
            avatar: None,
            text: format!("post {n}"),
            created: format!("2026-01-01T00:00:{:02}Z", n % 60),
            seen: seen.into(),
            links: Vec::new(),
            card: None,
            images: Vec::new(),
            liked: false,
            saved: false,
            mine: false,
            reply: false,
            likes: 0,
            reposts: 0,
            replies: 0,
        }
    }

    #[test]
    fn keeps_each_post_once_and_notes_changes() {
        let mut a = Archive::with(Did::from("did:plc:me"), Vec::new());
        a.keep([kept(1, "t1"), kept(2, "t1")]);
        assert!(a.dirty);
        a.dirty = false;
        a.keep([kept(1, "t2")]);
        assert!(!a.dirty, "nothing new");
        let mut liked = kept(1, "t3");
        liked.liked = true;
        a.keep([liked]);
        assert!(a.dirty);
        assert_eq!(a.len(), 2);
        assert!(a.posts()[0].liked);
        assert_eq!(a.posts()[0].seen, "t1");
    }

    #[test]
    fn the_oldest_plain_posts_go_first() {
        let mut posts: Vec<Kept> = (0..MAX_KEPT).map(|n| kept(n, "2026-01-02")).collect();
        posts[0].seen = "2026-01-01".into();
        posts[1].seen = "2026-01-01".into();
        posts[1].saved = true;
        let mut a = Archive::with(Did::from("did:plc:me"), posts);
        a.keep([kept(MAX_KEPT, "2026-01-03")]);
        assert_eq!(a.len(), MAX_KEPT);
        assert!(!a.at.contains_key("at://did:plc:a/app.bsky.feed.post/0"));
        assert!(
            a.at.contains_key("at://did:plc:a/app.bsky.feed.post/1"),
            "saved"
        );
        let last = a.at[&format!("at://did:plc:a/app.bsky.feed.post/{MAX_KEPT}")];
        assert_eq!(a.posts()[last].text, format!("post {MAX_KEPT}"));
    }

    #[test]
    fn searches_newest_first() {
        let a = Archive::with(
            Did::from("did:plc:me"),
            vec![kept(1, "t"), kept(3, "t"), kept(2, "t")],
        );
        let found = a.search(&Query::parse("post"));
        let texts: Vec<&str> = found.iter().map(|k| k.text.as_str()).collect();
        assert_eq!(texts, ["post 3", "post 2", "post 1"]);
        assert_eq!(a.search(&Query::parse("post 2")).len(), 1);
    }
}
