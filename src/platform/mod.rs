//! Where the operating system shows through. Each difference between Windows, macOS
//! and Linux lives here, behind one function with a version per OS, so feature code
//! never needs a `cfg`.

pub mod clipboard;
pub mod fonts;
pub mod locale;
pub mod notify;
pub mod paths;
pub mod tray;
