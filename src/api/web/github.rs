//! BiFrost's own GitHub repository: the newest release, for the opt-in update check,
//! and the page that opens a new issue with the details a report needs already filled
//! in. Nothing here is sent anywhere until the person asks.

use crate::api::ApiError;
use crate::api::client::ApiClient;
use crate::net::{Transport, shared};

/// Where BiFrost's code, releases and issues live.
pub const REPOSITORY: &str = "https://github.com/nickmorell/bifrost";

/// The version this build of BiFrost is.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A release newer than this build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The version, without the tag's leading `v`.
    pub version: String,
    /// The release's page, where its downloads are.
    pub url: String,
}

/// GitHub's REST API, without signing in.
pub struct GitHub<'a>(pub &'a dyn Transport);

impl ApiClient for GitHub<'_> {
    fn base_url(&self) -> &'static str {
        "https://api.github.com/repos/nickmorell/bifrost"
    }

    fn transport(&self) -> &dyn Transport {
        self.0
    }

    fn headers(&self) -> Vec<(String, String)> {
        vec![("Accept".into(), "application/vnd.github+json".into())]
    }
}

impl GitHub<'_> {
    /// The newest published release when it is newer than `current`. Drafts and
    /// pre-releases are never the newest.
    pub async fn newer_than(&self, current: &str) -> Result<Option<Release>, ApiError> {
        #[derive(serde::Deserialize)]
        struct Latest {
            tag_name: String,
            html_url: String,
        }
        let latest: Latest = self.get("/releases/latest", &[]).await?;
        let version = latest.tag_name.trim_start_matches('v');
        Ok(is_newer(version, current).then(|| Release {
            version: version.to_string(),
            url: latest.html_url,
        }))
    }
}

/// Asks GitHub whether a newer BiFrost is out.
pub async fn newer_release() -> Result<Option<Release>, ApiError> {
    GitHub(shared()).newer_than(VERSION).await
}

/// Whether `version` comes after `current`. Both are `major.minor.patch`; anything
/// after a `-` or `+` is ignored, and a version that doesn't read as one is never newer.
fn is_newer(version: &str, current: &str) -> bool {
    fn parts(v: &str) -> Option<[u64; 3]> {
        let core = v.split(['-', '+']).next()?;
        let mut it = core.split('.').map(|n| n.parse::<u64>().ok());
        let out = [it.next()??, it.next()??, it.next()??];
        it.next().is_none().then_some(out)
    }
    matches!((parts(version), parts(current)), (Some(v), Some(c)) if v > c)
}

/// The page that opens the repository's bug form, with this build's version and, where
/// it can be told, the system filled in. The person reads and sends it themselves.
pub fn new_issue_url(os: &str, arch: &str) -> String {
    let mut fields = vec![("template", "bug.yml"), ("version", VERSION)];
    // The form's "Operating system" choices; Windows 10 and 11 can't be told apart here.
    let system = match (os, arch) {
        ("macos", "aarch64") => Some("macOS (Apple silicon)"),
        ("macos", _) => Some("macOS (Intel)"),
        ("linux", _) => Some("Linux"),
        _ => None,
    };
    if let Some(system) = system {
        fields.push(("os", system));
    }
    match url::Url::parse_with_params(&format!("{REPOSITORY}/issues/new"), &fields) {
        Ok(url) => url.into(),
        Err(_) => format!("{REPOSITORY}/issues/new/choose"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fake::Fake;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.10.0"));
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.1"));
        assert!(!is_newer("0.2.0-beta", "0.2.0"));
        assert!(!is_newer("nightly", "0.1.0"));
        assert!(!is_newer("0.2", "0.1.0"));
        assert!(!is_newer("0.2.0.1", "0.1.0"));
    }

    #[tokio::test]
    async fn finds_a_newer_release() {
        let net = Fake::default()
            .reply(
                200,
                r#"{"tag_name":"v0.2.0","html_url":"https://github.com/nickmorell/bifrost/releases/tag/v0.2.0","draft":false}"#,
            )
            .reply(200, r#"{"tag_name":"v0.1.0","html_url":"https://x"}"#)
            .reply(404, r#"{"message":"Not Found"}"#);
        let gh = GitHub(&net);
        assert_eq!(
            gh.newer_than("0.1.0").await.unwrap(),
            Some(Release {
                version: "0.2.0".into(),
                url: "https://github.com/nickmorell/bifrost/releases/tag/v0.2.0".into(),
            })
        );
        let seen = &net.seen()[0];
        assert_eq!(
            seen.url.as_str(),
            "https://api.github.com/repos/nickmorell/bifrost/releases/latest"
        );
        assert!(
            seen.headers
                .contains(&("Accept".into(), "application/vnd.github+json".into()))
        );
        assert_eq!(gh.newer_than("0.1.0").await.unwrap(), None);
        assert!(gh.newer_than("0.1.0").await.is_err());
    }

    #[test]
    fn fills_in_the_bug_form() {
        let field = |url: &str, name: &str| {
            url::Url::parse(url)
                .unwrap()
                .query_pairs()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.into_owned())
        };
        let mac = new_issue_url("macos", "aarch64");
        assert!(
            mac.starts_with("https://github.com/nickmorell/bifrost/issues/new?template=bug.yml")
        );
        assert_eq!(field(&mac, "version").as_deref(), Some(VERSION));
        assert_eq!(field(&mac, "os").as_deref(), Some("macOS (Apple silicon)"));
        let intel = new_issue_url("macos", "x86_64");
        assert_eq!(field(&intel, "os").as_deref(), Some("macOS (Intel)"));
        assert_eq!(
            field(&new_issue_url("linux", "x86_64"), "os").as_deref(),
            Some("Linux")
        );
        let windows = new_issue_url("windows", "x86_64");
        assert_eq!(field(&windows, "version").as_deref(), Some(VERSION));
        assert_eq!(
            field(&windows, "os"),
            None,
            "Windows 10 or 11 is the person's to pick"
        );
    }
}
