//! Avatars and post thumbnails kept on disk between launches. Bluesky's image addresses
//! name the picture's content, so a stored copy never goes stale.

use crate::platform::paths;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

/// Space the cache may use before the least recently used files are removed.
const CAP: u64 = 200 * 1024 * 1024;
/// Removing stops once usage is back under this.
const LOW_WATER: u64 = CAP / 4 * 3;
/// Files written between checks of the cache's size.
const TRIM_EVERY: usize = 300;
static WRITES: AtomicUsize = AtomicUsize::new(0);

fn dir() -> Option<PathBuf> {
    Some(paths::cache_dir()?.join("images"))
}

/// Only the small pictures seen over and over are kept: avatars and thumbnails.
fn cacheable(url: &str) -> bool {
    url.strip_prefix("https://cdn.bsky.app/img/")
        .is_some_and(|rest| {
            ["avatar/", "avatar_thumbnail/", "feed_thumbnail/"]
                .iter()
                .any(|kind| rest.starts_with(kind))
        })
}

/// The file name for `url`: a hash, so nothing from the address reaches the path.
fn name(url: &str) -> String {
    // FNV-1a: stable across builds, unlike the standard library's hasher.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in url.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    format!("{h:016x}")
}

/// The stored copy of `url`, if there is one.
pub async fn read(url: &str) -> Option<Vec<u8>> {
    if !cacheable(url) {
        return None;
    }
    let path = dir()?.join(name(url));
    tokio::task::spawn_blocking(move || read_in(&path))
        .await
        .ok()
        .flatten()
}

fn read_in(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    // Marks it as recently used, so trimming keeps it.
    if let Ok(f) = std::fs::File::options().write(true).open(path) {
        // If not, it is trimmed a little sooner, nothing worse.
        let _ = f.set_modified(SystemTime::now());
    }
    Some(bytes)
}

/// Keeps a downloaded copy of `url`. Failures only cost a download next time.
pub async fn write(url: &str, bytes: Vec<u8>) {
    if !cacheable(url) {
        return;
    }
    let Some(dir) = dir() else { return };
    let name = name(url);
    // Failing to start the thread only costs a download next time, like any failure.
    let _ = tokio::task::spawn_blocking(move || {
        write_in(&dir, &name, &bytes);
        if WRITES.fetch_add(1, Ordering::Relaxed) % TRIM_EVERY == TRIM_EVERY - 1 {
            trim_in(&dir, CAP, LOW_WATER);
        }
    })
    .await;
}

fn write_in(dir: &Path, name: &str, bytes: &[u8]) {
    if paths::create_dir(dir).is_err() {
        return;
    }
    // Written whole and then renamed, so a reader never sees half a file.
    let tmp = dir.join(format!("{name}.tmp"));
    if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, dir.join(name)).is_err() {
        // A leftover is overwritten by the next try or trimmed with the rest.
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Brings the cache back under its cap. Run at startup, off the interface thread.
pub fn trim() {
    if let Some(dir) = dir() {
        trim_in(&dir, CAP, LOW_WATER);
    }
}

fn trim_in(dir: &Path, cap: u64, low: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(SystemTime, u64, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some((meta.modified().ok()?, meta.len(), e.path()))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    if total <= cap {
        return;
    }
    files.sort_unstable_by_key(|f| f.0);
    for (_, len, path) in files {
        if total <= low {
            break;
        }
        if std::fs::remove_file(path).is_ok() {
            total -= len;
        }
    }
}

/// Removes every stored picture.
pub fn clear() {
    if let Some(dir) = dir() {
        // The cache folder goes too, last, and that failure is the one reported.
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_avatars_and_thumbnails() {
        assert!(cacheable(
            "https://cdn.bsky.app/img/avatar_thumbnail/plain/did:plc:a/b@jpeg"
        ));
        assert!(cacheable(
            "https://cdn.bsky.app/img/feed_thumbnail/plain/did:plc:a/b@jpeg"
        ));
        assert!(!cacheable(
            "https://cdn.bsky.app/img/feed_fullsize/plain/did:plc:a/b@jpeg"
        ));
        assert!(!cacheable("https://example.com/img/avatar/x"));
        assert_eq!(name("a").len(), 16);
        assert_ne!(name("a"), name("b"));
    }

    #[test]
    fn trims_least_recently_used_first() {
        let dir = std::env::temp_dir().join(format!("bifrost-imgcache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (i, n) in ["old", "mid", "new"].iter().enumerate() {
            write_in(&dir, n, &[0u8; 100]);
            let f = std::fs::File::options()
                .write(true)
                .open(dir.join(n))
                .unwrap();
            f.set_modified(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(i as u64 * 100))
                .unwrap();
        }
        // Reading "old" makes it the most recently used.
        assert!(read_in(&dir.join("old")).is_some());
        trim_in(&dir, 250, 150);
        assert!(dir.join("old").exists());
        assert!(!dir.join("mid").exists());
        assert!(!dir.join("new").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
