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
    /// Free-form, such as "she/her".
    pub pronouns: Option<String>,
    /// The older single website field, which some profiles still set.
    pub website: Option<String>,
    /// Links they chose to show, in their order.
    #[serde(default)]
    pub links: Vec<ProfileLink>,
}

/// A link shown on a profile, from its owner's `app.bsky.actor.link` record.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLink {
    /// Its `app.bsky.actor.link` record; `None` for the old website field.
    #[serde(flatten)]
    pub record: Option<StrongRef>,
    pub url: String,
    pub title: Option<String>,
    /// The destination site's icon.
    pub icon: Option<String>,
    #[serde(default)]
    pub labels: Vec<Label>,
}

impl ProfileLink {
    /// Its title, or the site it goes to when it has none.
    pub fn label(&self) -> String {
        match self.title.as_deref().map(str::trim) {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => site_of(&self.url),
        }
    }
}

/// The site an address is on, without `www.`, or the address itself.
fn site_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_string())
        })
        .unwrap_or_else(|| url.to_string())
}

impl Profile {
    /// The links to show: https only, none that moderators hid, then the old website
    /// field when no link already goes there.
    pub fn shown_links(&self) -> Vec<ProfileLink> {
        let hidden = |l: &ProfileLink| l.labels.iter().any(|x| !x.neg && x.val.starts_with('!'));
        let mut links: Vec<ProfileLink> = self
            .links
            .iter()
            .filter(|l| l.url.starts_with("https://") && !hidden(l))
            .cloned()
            .collect();
        if let Some(site) = self.website.as_deref().map(str::trim) {
            let same = |l: &ProfileLink| l.url.trim_end_matches('/') == site.trim_end_matches('/');
            if site.starts_with("https://") && !links.iter().any(same) {
                links.push(ProfileLink {
                    record: None,
                    url: site.to_string(),
                    title: None,
                    icon: None,
                    labels: Vec::new(),
                });
            }
        }
        links
    }

    /// Their pronouns, when set.
    pub fn pronouns(&self) -> Option<&str> {
        self.pronouns
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
    }

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

    #[test]
    fn links_shown_are_safe_and_in_order() {
        let p: Profile = serde_json::from_value(json!({
            "did": "did:plc:a", "handle": "a.test", "pronouns": " they/them ",
            "website": "https://www.example.com/",
            "links": [
                {"uri": "at://a/app.bsky.actor.link/1", "cid": "c", "url": "https://www.example.com", "title": ""},
                {"uri": "at://a/app.bsky.actor.link/2", "cid": "c", "url": "javascript:alert(1)"},
                {"uri": "at://a/app.bsky.actor.link/3", "cid": "c", "url": "https://shop.test/x",
                 "labels": [{"src": "did:plc:mod", "val": "!hide"}]},
                {"uri": "at://a/app.bsky.actor.link/4", "cid": "c", "url": "https://blog.test", "title": "My blog"}
            ]
        }))
        .unwrap();
        let shown: Vec<String> = p.shown_links().iter().map(ProfileLink::label).collect();
        assert_eq!(shown, ["example.com", "My blog"]);
        let first = p.links[0].record.as_ref().map(|r| r.uri.as_str());
        assert_eq!(first, Some("at://a/app.bsky.actor.link/1"));
        assert_eq!(p.pronouns(), Some("they/them"));

        let old: Profile = serde_json::from_value(json!({
            "did": "did:plc:a", "handle": "a.test", "website": "https://old.test"
        }))
        .unwrap();
        assert_eq!(old.shown_links()[0].url, "https://old.test");
    }
}
