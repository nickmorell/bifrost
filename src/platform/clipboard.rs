//! Reading images from the clipboard.

use std::path::PathBuf;

/// Images on the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardImages {
    /// A picture, such as a screenshot, as BMP bytes.
    Picture(Vec<u8>),
    /// Files copied in the file manager.
    Files(Vec<PathBuf>),
}

/// What image the clipboard holds, if any. Blocks while the clipboard is read, so
/// call it off the UI thread. Only Windows is read so far; elsewhere this is `None`.
pub fn images() -> Option<ClipboardImages> {
    #[cfg(windows)]
    {
        use clipboard_win::{formats, get_clipboard};
        if let Ok(files) = get_clipboard::<Vec<PathBuf>, _>(formats::FileList) {
            return Some(ClipboardImages::Files(files));
        }
        get_clipboard::<Vec<u8>, _>(formats::Bitmap)
            .ok()
            .map(ClipboardImages::Picture)
    }
    #[cfg(not(windows))]
    {
        None
    }
}
