//! The ATstore directory as BiFrost last read it, kept in the cache folder so the
//! ATstore screen opens at once, and offline. It can be thrown away and read again.

use super::later::Later;
use crate::model::Catalog;
use crate::platform::paths;
use std::path::{Path, PathBuf};

fn path() -> Option<PathBuf> {
    Some(paths::cache_dir()?.join("atstore.json"))
}

/// The kept directory, or `None` if there is none or it can't be read.
pub fn load() -> Option<Catalog> {
    read(&path()?)
}

/// Keeps the directory, in the background.
pub fn save(catalog: &Catalog) {
    static FILE: Later = Later::new("the ATstore directory");
    if let (Some(path), Ok(json)) = (path(), serde_json::to_vec(catalog)) {
        FILE.write(path, json);
    }
}

fn read(path: &Path) -> Option<Catalog> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_or_broken_file_reads_as_none() {
        let dir = std::env::temp_dir().join(format!("bifrost-atstore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("atstore.json");
        assert_eq!(read(&file), None);
        std::fs::write(&file, b"not json").unwrap();
        assert_eq!(read(&file), None);
        let c = Catalog {
            fetched: "2026-10-14T00:00:00Z".into(),
            ..Catalog::default()
        };
        std::fs::write(&file, serde_json::to_vec(&c).unwrap()).unwrap();
        assert_eq!(read(&file), Some(c));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
