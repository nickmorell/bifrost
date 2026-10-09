//! BiFrost's folders on this system.

use std::path::{Path, PathBuf};

/// Settings, drafts and the account list: `%APPDATA%\bifrost` on Windows,
/// `~/Library/Application Support/bifrost` on macOS, `~/.config/bifrost` on Linux.
pub fn config_dir() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("bifrost"))
}

/// Files that can be thrown away and fetched again: `%LOCALAPPDATA%\bifrost` on
/// Windows, `~/Library/Caches/bifrost` on macOS, `~/.cache/bifrost` on Linux.
pub fn cache_dir() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("bifrost"))
}

/// Creates `dir` (and the folders above it) for BiFrost's own files. On macOS and
/// Linux, BiFrost's config and cache folders are made readable by you alone, as they
/// hold your drafts, settings and account list; on Windows they are already yours,
/// under your user profile.
pub fn create_dir(dir: &Path) -> std::io::Result<()> {
    let roots: Vec<PathBuf> = [config_dir(), cache_dir()].into_iter().flatten().collect();
    create_under(dir, &roots)
}

/// Creates `dir`, and makes whichever of `roots` holds it private.
fn create_under(dir: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for root in roots.iter().filter(|r| dir.starts_with(r)) {
        only_mine(root)?;
    }
    Ok(())
}

#[cfg(unix)]
fn only_mine(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "same signature as the Unix version, which can fail"
)]
fn only_mine(_: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Deletes both folders and everything in them, for "Remove all BiFrost data". A
/// folder that is already gone is fine; any other failure is returned once both
/// were tried.
pub fn remove_all() -> std::io::Result<()> {
    let mut result = Ok(());
    for dir in [config_dir(), cache_dir()].into_iter().flatten() {
        match std::fs::remove_dir_all(dir) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => result = Err(e),
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn our_folders_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("bifrost-private-{}", std::process::id()));
        create_under(&root.join("drafts"), std::slice::from_ref(&root)).unwrap();
        let mode = std::fs::metadata(&root).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folders_are_named_after_the_app() {
        for dir in [config_dir(), cache_dir()].into_iter().flatten() {
            assert!(dir.ends_with("bifrost"));
        }
    }
}
