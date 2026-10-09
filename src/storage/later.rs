//! Writing a file in the background, so the window never waits on the disk.

use crate::platform::paths;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use tokio::sync::watch;

/// A background write that failed, for the app to tell people about.
#[derive(Debug, Clone)]
pub struct Failed {
    /// What the file holds, such as "settings".
    pub what: &'static str,
    pub error: String,
}

impl std::fmt::Display for Failed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not save {}: {}", self.what, self.error)
    }
}

static FAILED: LazyLock<watch::Sender<Option<Failed>>> = LazyLock::new(|| watch::Sender::new(None));

/// Hears about every background write that fails from now on.
pub fn failures() -> watch::Receiver<Option<Failed>> {
    FAILED.subscribe()
}

/// Background writes of one file, in order: a write that finishes late never
/// overwrites a newer one.
pub struct Later {
    /// What the file holds, for the message when a write fails.
    what: &'static str,
    seq: AtomicU64,
    written: Mutex<u64>,
}

impl Later {
    pub const fn new(what: &'static str) -> Self {
        Self {
            what,
            seq: AtomicU64::new(0),
            written: Mutex::new(0),
        }
    }

    /// Writes `bytes` to `path` on another thread.
    pub fn write(&'static self, path: PathBuf, bytes: Vec<u8>) {
        let seq = self.next();
        std::thread::spawn(move || self.store(seq, &path, &bytes));
    }

    /// Writes `bytes` to `path` before returning, for when BiFrost is about to quit.
    pub fn write_now(&self, path: &Path, bytes: &[u8]) {
        self.store(self.next(), path, bytes);
    }

    fn next(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Writes a temporary file and renames it, so a crash never leaves half a file.
    /// A failure is passed on to [`failures`].
    fn store(&self, seq: u64, path: &Path, bytes: &[u8]) {
        let mut written = self
            .written
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if seq <= *written {
            return;
        }
        let tmp = path.with_extension("json.tmp");
        let stored = path
            .parent()
            .map_or(Ok(()), paths::create_dir)
            .and_then(|()| std::fs::write(&tmp, bytes))
            .and_then(|()| std::fs::rename(&tmp, path));
        match stored {
            Ok(()) => *written = seq,
            Err(e) => {
                FAILED.send_replace(Some(Failed {
                    what: self.what,
                    error: e.to_string(),
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_write_is_reported_and_a_good_one_lands() {
        let dir = std::env::temp_dir().join(format!("bifrost-later-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut heard = failures();
        // A file where the folder should be: the write can't happen.
        std::fs::write(dir.join("blocked"), b"").unwrap();
        let file = Later::new("test file");
        file.write_now(&dir.join("blocked/x.json"), b"{}");
        assert!(heard.has_changed().unwrap());
        let failed = heard.borrow_and_update().clone().unwrap();
        assert_eq!(failed.what, "test file");
        assert!(failed.to_string().starts_with("Could not save test file: "));

        file.write_now(&dir.join("ok.json"), b"{}");
        assert_eq!(std::fs::read(dir.join("ok.json")).unwrap(), b"{}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
