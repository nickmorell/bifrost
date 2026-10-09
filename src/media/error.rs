//! Why a picture or video can't be used.

/// What went wrong with one image or video.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MediaError {
    #[error("not a supported image ({0})")]
    Unsupported(String),
    /// Bigger than the limits allow, before or after decoding.
    #[error("too large ({0})")]
    TooLarge(String),
    #[error("image is too large even after shrinking")]
    TooLargeToShrink,
    #[error("couldn't encode the image ({0})")]
    Encode(String),
    /// A video over Bluesky's size limit, in MB.
    #[error("is {0} MB; Bluesky takes videos up to 100 MB.")]
    VideoTooBig(u64),
    #[error("is longer than Bluesky's 3 minute limit.")]
    VideoTooLong,
    /// The file couldn't be read.
    #[error("{0}")]
    Read(String),
}

/// A file that can't be used, with its name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub struct FileError {
    pub name: String,
    pub error: MediaError,
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.error {
            // These read as a sentence about the file.
            MediaError::VideoTooBig(_) | MediaError::VideoTooLong => {
                write!(f, "{} {}", self.name, self.error)
            }
            _ => write!(f, "{}: {}", self.name, self.error),
        }
    }
}

impl MediaError {
    /// The same problem, naming the file.
    pub fn in_file(self, name: impl Into<String>) -> FileError {
        FileError {
            name: name.into(),
            error: self,
        }
    }
}
