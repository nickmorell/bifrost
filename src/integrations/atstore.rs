//! ATstore (atstore.fyi), the directory of apps and services built on the AT
//! Protocol. Turning it on adds the ATstore screen to the sidebar, for browsing the
//! directory by tag or search and opening an app's page.

use super::{Group, Integration};

pub struct Atstore;

/// The id ATstore's switch is kept under.
pub const ID: &str = "atstore";

impl Integration for Atstore {
    fn id(&self) -> &'static str {
        ID
    }

    fn name(&self) -> &'static str {
        "ATstore"
    }

    fn description(&self) -> &'static str {
        "Browse apps and services built on the AT Protocol from atstore.fyi, in an ATstore screen in the sidebar."
    }

    fn group(&self) -> Group {
        Group::Service
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::settings::Settings;

    #[test]
    fn is_a_service_that_starts_off() {
        assert_eq!(Atstore.group(), Group::Service);
        assert!(!Settings::default().integration_enabled(ID));
    }
}
