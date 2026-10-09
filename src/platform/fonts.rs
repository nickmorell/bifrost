//! The system's own fonts.

/// The system's interface font, for the "Reading font" setting. Not bundled: every
/// system has one, and a missing name falls back to another sans-serif font.
#[cfg(windows)]
pub const SANS_FAMILY: &str = "Segoe UI";
#[cfg(target_os = "macos")]
pub const SANS_FAMILY: &str = "Helvetica Neue";
#[cfg(not(any(windows, target_os = "macos")))]
pub const SANS_FAMILY: &str = "Noto Sans";
