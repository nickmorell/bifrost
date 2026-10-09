//! Pop-up notifications from the operating system: Windows toasts, macOS Notification
//! Center and the freedesktop notification service on Linux, all through notify-rust.

use std::fmt;

/// One notification: a title and a line or two of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub body: String,
}

/// Why a notification could not be shown, such as no notification service running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyError(pub String);

impl fmt::Display for NotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NotifyError {}

/// Something that shows notifications. Showing can block for a moment (it talks to an
/// OS service), so callers run it off the UI thread.
pub trait Notifier: Send + Sync {
    fn show(&self, notice: &Notice) -> Result<(), NotifyError>;
}

/// The operating system's notifications.
#[derive(Debug, Default, Clone, Copy)]
pub struct System;

impl Notifier for System {
    fn show(&self, notice: &Notice) -> Result<(), NotifyError> {
        let mut notification = notify_rust::Notification::new();
        notification
            .appname("BiFrost")
            .summary(&notice.title)
            .body(&plain(&notice.body));
        #[cfg(windows)]
        if let Some(id) = windows::app_id() {
            notification.app_id(id);
        }
        notification
            .show()
            .map(drop)
            .map_err(|e| NotifyError(e.to_string()))
    }
}

/// The body as the OS shows it word for word. Linux notification services read the
/// body as markup, so a message's `<a href=…>` or `<img src=…>` would turn into a
/// link or a picture fetched from anywhere; there it is escaped. Windows and macOS
/// show it as given.
#[cfg(target_os = "linux")]
fn plain(body: &str) -> String {
    body.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(not(target_os = "linux"))]
fn plain(body: &str) -> String {
    body.to_string()
}

/// Removes what showing notifications left on this system, for "Remove all BiFrost
/// data": on Windows, BiFrost's registered app id. Its icon goes with the cache folder.
pub fn forget() {
    #[cfg(windows)]
    windows::forget();
}

/// Windows names a notification's sender after an app id registered for the user.
/// Without one, notify-rust borrows PowerShell's, and toasts say "Windows PowerShell".
#[cfg(windows)]
mod windows {
    use std::sync::OnceLock;
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    const APP_ID: &str = "BiFrost.Desktop";

    /// BiFrost's app id, registered the first time it is asked for; `None` when the
    /// registry can't be written, so toasts fall back to PowerShell's name.
    pub(super) fn app_id() -> Option<&'static str> {
        static REGISTERED: OnceLock<bool> = OnceLock::new();
        REGISTERED
            .get_or_init(|| register().is_ok())
            .then_some(APP_ID)
    }

    /// Deletes the app id's registry entry.
    pub(super) fn forget() {
        // Usually it is not there at all; that is the same as removed.
        let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(key_path());
    }

    fn key_path() -> String {
        format!(r"Software\Classes\AppUserModelId\{APP_ID}")
    }

    /// The per-user entry Windows reads a toast's name and icon from. Rewritten each
    /// run, so it follows the cache folder if that moves.
    fn register() -> std::io::Result<()> {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(key_path())?;
        key.set_value("DisplayName", &"BiFrost")?;
        if let Some(icon) = write_icon() {
            key.set_value("IconUri", &icon.as_os_str())?;
        }
        Ok(())
    }

    /// The tray's icon, without a badge, saved as a PNG for Windows to read.
    fn write_icon() -> Option<std::path::PathBuf> {
        let dir = super::super::paths::cache_dir()?;
        super::super::paths::create_dir(&dir).ok()?;
        let path = dir.join("notification-icon.png");
        let side = super::super::tray::icon::SIZE as u32;
        let pixels = super::super::tray::icon::pixels(0);
        image::save_buffer(&path, &pixels, side, side, image::ExtendedColorType::Rgba8).ok()?;
        Some(path)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn markup_shows_as_text_on_linux() {
        assert_eq!(
            super::plain(r#"<img src="https://x/t.png"> & <b>hi</b>"#),
            r#"&lt;img src="https://x/t.png"&gt; &amp; &lt;b&gt;hi&lt;/b&gt;"#
        );
    }
}
