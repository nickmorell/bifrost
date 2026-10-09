//! Your privacy requests to Bluesky: out of Discover, and hidden from logged-out
//! visitors.

use super::{ApiError, Client, PROFILE};
use crate::model::{Privacy, VISIBILITY_DECLARATION, set_logged_out};

impl Client {
    /// What you have asked of Bluesky now.
    pub async fn privacy(&self) -> Result<Privacy, ApiError> {
        let declaration = self.my_record(VISIBILITY_DECLARATION, "self").await?;
        let profile = self.my_record(PROFILE.0, PROFILE.1).await?;
        Ok(Privacy::from_records(
            declaration.as_ref().map(|d| &d.0),
            profile.as_ref().map(|p| &p.0),
        ))
    }

    /// Asks for your posts to be kept out of Discover and other algorithmic feeds.
    pub async fn set_hide_from_discover(&self, hide: bool) -> Result<(), ApiError> {
        let swap = self
            .my_record(VISIBILITY_DECLARATION, "self")
            .await?
            .map(|r| r.1);
        self.put_record(
            VISIBILITY_DECLARATION,
            "self",
            Privacy::declaration(hide),
            swap,
        )
        .await?;
        Ok(())
    }

    /// Asks for your profile and posts to be hidden from people who aren't signed in.
    pub async fn set_hide_logged_out(&self, hide: bool) -> Result<(), ApiError> {
        self.edit_profile_record(|r| set_logged_out(r, hide)).await
    }
}
