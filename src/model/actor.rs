//! People: profiles and how you relate to them.

use super::{ActorStatus, AtUri, Did, Label, Mine, StrongRef};
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorViewer {
    #[serde(default)]
    pub muted: bool,
    pub blocking: Option<Mine>,
    #[serde(default)]
    pub blocked_by: bool,
    pub following: Option<Mine>,
    pub followed_by: Option<AtUri>,
}

/// Basic profile, as embedded in posts and notifications.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub did: Did,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub viewer: ActorViewer,
    /// What they are up to, such as streaming live.
    pub status: Option<Box<ActorStatus>>,
}

/// Other things linked to a profile.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Associated {
    /// Where to message them on Germ, an end-to-end encrypted messenger.
    pub germ: Option<Germ>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Germ {
    pub message_me_url: String,
    /// "everyone", or "usersIFollow" for only the people they follow.
    pub show_button_to: String,
}

impl Author {
    /// Display name if set and non-empty, otherwise the handle.
    pub fn name(&self) -> &str {
        match self.display_name.as_deref() {
            Some(n) if !n.trim().is_empty() => n,
            _ => &self.handle,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub did: Did,
    pub handle: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    #[serde(default)]
    pub followers_count: u64,
    #[serde(default)]
    pub follows_count: u64,
    #[serde(default)]
    pub posts_count: u64,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub viewer: ActorViewer,
    /// The post shown first on the profile.
    pub pinned_post: Option<StrongRef>,
    /// What they are up to, such as streaming live.
    pub status: Option<Box<ActorStatus>>,
    #[serde(default)]
    pub associated: Associated,
}

impl Profile {
    /// Their Germ address, when they offer it to you: to everyone, or only to people
    /// they follow. Only https addresses are offered.
    pub fn germ(&self) -> Option<&str> {
        let germ = self.associated.germ.as_ref()?;
        let shown = match germ.show_button_to.as_str() {
            "everyone" => true,
            "usersIFollow" => self.viewer.followed_by.is_some(),
            _ => false,
        };
        (shown && germ.message_me_url.starts_with("https://")).then_some(&germ.message_me_url)
    }

    pub fn name(&self) -> &str {
        match self.display_name.as_deref() {
            Some(n) if !n.trim().is_empty() => n,
            _ => &self.handle,
        }
    }
}

/// Which list of accounts a people screen shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PeopleKind {
    Followers,
    Following,
    Muted,
    Blocked,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn profile(germ: serde_json::Value, followed_by: bool) -> Profile {
        let mut p = json!({"did": "did:plc:a", "handle": "a.test",
                           "associated": {"germ": germ}});
        if followed_by {
            p["viewer"] = json!({"followedBy": "at://did:plc:a/app.bsky.graph.follow/1"});
        }
        serde_json::from_value(p).unwrap()
    }

    #[test]
    fn germ_is_offered_as_its_owner_chose() {
        let to = |who: &str, url: &str| json!({"messageMeUrl": url, "showButtonTo": who});
        let url = "https://landing.germnetwork.com/x";
        assert_eq!(profile(to("everyone", url), false).germ(), Some(url));
        assert_eq!(profile(to("usersIFollow", url), false).germ(), None);
        assert_eq!(profile(to("usersIFollow", url), true).germ(), Some(url));
        assert_eq!(profile(to("everyone", "javascript:x"), false).germ(), None);
        let none: Profile =
            serde_json::from_value(json!({"did": "did:plc:a", "handle": "a"})).unwrap();
        assert_eq!(none.germ(), None);
    }
}
