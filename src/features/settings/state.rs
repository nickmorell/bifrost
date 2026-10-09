//! What the Settings screen remembers between messages.

/// The Settings screen's own state. Preferences are not in here: the shell owns those.
#[derive(Debug, Default)]
pub struct Screen {
    /// "Remove all BiFrost data" was pressed and is asking before it acts.
    pub(super) confirm_remove_all: bool,
}

impl Screen {
    /// Whether the remove-everything card is asking for confirmation.
    pub fn confirming(&self) -> bool {
        self.confirm_remove_all
    }
}
