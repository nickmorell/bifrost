//! GitHub repositories, issues and pull requests, with their stars, language and
//! state read from GitHub's public API without signing in.

use super::{Details, Fact, Integration, Lookup, Shape, Shown, card, host, parts};
use serde::Deserialize;
use std::time::Duration;
use url::Url;

pub struct GitHub;

/// What a link points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind<'a> {
    Repo(&'a str, &'a str),
    Issue(&'a str, &'a str, u64),
    Pull(&'a str, &'a str, u64),
}

impl Integration for GitHub {
    fn id(&self) -> &'static str {
        "github"
    }

    fn name(&self) -> &'static str {
        "GitHub"
    }

    fn description(&self) -> &'static str {
        "Show GitHub repositories, issues and pull requests as cards, with stars and state from GitHub."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        let what = match kind(url)? {
            Kind::Repo(..) => "Repository",
            Kind::Issue(..) => "Issue",
            Kind::Pull(..) => "Pull request",
        };
        Some(card("GitHub", what, "Open", Shape::Cover).open(url, title))
    }

    fn lookup(&self, url: &Url) -> Option<Lookup> {
        const API: &str = "https://api.github.com/repos";
        let (url, keep) = match kind(url)? {
            Kind::Repo(o, r) => (format!("{API}/{o}/{r}"), DAY),
            Kind::Issue(o, r, n) => (format!("{API}/{o}/{r}/issues/{n}"), HOUR),
            Kind::Pull(o, r, n) => (format!("{API}/{o}/{r}/pulls/{n}"), HOUR),
        };
        Some(Lookup { url, keep })
    }

    fn details(&self, url: &Url, body: &[u8]) -> Option<Details> {
        match kind(url)? {
            Kind::Repo(..) => Some(repo(serde_json::from_slice(body).ok()?)),
            Kind::Issue(o, r, n) | Kind::Pull(o, r, n) => {
                issue(serde_json::from_slice(body).ok()?, format!("{o}/{r} #{n}"))
            }
        }
    }
}

const HOUR: Duration = Duration::from_secs(60 * 60);
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// Pages of github.com itself, which look like an account's name.
const NOT_ACCOUNTS: &[&str] = &[
    "about",
    "apps",
    "collections",
    "customer-stories",
    "enterprise",
    "events",
    "explore",
    "features",
    "issues",
    "login",
    "marketplace",
    "new",
    "notifications",
    "orgs",
    "pricing",
    "pulls",
    "search",
    "security",
    "settings",
    "sponsors",
    "topics",
    "trending",
];

fn kind(url: &Url) -> Option<Kind<'_>> {
    if host(url)? != "github.com" {
        return None;
    }
    let p = parts(url);
    let (owner, repo) = (*p.first()?, *p.get(1)?);
    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    if !is_account(owner) || !is_repo(repo) {
        return None;
    }
    let number = |n: &str| n.parse::<u64>().ok();
    Some(match p.get(2..)? {
        [] => Kind::Repo(owner, repo),
        ["issues", n] => Kind::Issue(owner, repo, number(n)?),
        ["pull", n, ..] => Kind::Pull(owner, repo, number(n)?),
        _ => return None,
    })
}

/// A GitHub account: letters, digits and inner single hyphens, at most 39 long.
fn is_account(name: &str) -> bool {
    (1..=39).contains(&name.len())
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !NOT_ACCOUNTS.contains(&name.to_ascii_lowercase().as_str())
}

/// A repository name: letters, digits, `-`, `_` and `.`, but not only dots.
fn is_repo(name: &str) -> bool {
    (1..=100).contains(&name.len())
        && name.bytes().any(|b| b != b'.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

#[derive(Deserialize)]
struct Owner {
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct Repo {
    full_name: String,
    description: Option<String>,
    #[serde(default)]
    stargazers_count: u64,
    language: Option<String>,
    #[serde(default)]
    archived: bool,
    owner: Option<Owner>,
}

fn repo(r: Repo) -> Details {
    let mut facts = vec![Fact::Count(r.stargazers_count, "stars")];
    facts.extend(r.language.map(Fact::Text));
    if r.archived {
        facts.push(Fact::Text("Archived".into()));
    }
    Details {
        title: Some(r.full_name),
        text: r.description.filter(|d| !d.trim().is_empty()),
        thumbnail_url: r.owner.and_then(|o| o.avatar_url),
        facts,
        live: false,
    }
}

/// An issue or a pull request; GitHub answers both with these fields.
#[derive(Deserialize)]
struct Issue {
    title: String,
    state: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    merged: bool,
    #[serde(default)]
    comments: u64,
    user: Option<Owner>,
}

fn issue(i: Issue, place: String) -> Option<Details> {
    let state = match (i.state.as_str(), i.merged, i.draft) {
        (_, true, _) => "Merged",
        ("open", _, true) => "Draft",
        ("open", ..) => "Open",
        ("closed", ..) => "Closed",
        _ => return None,
    };
    Some(Details {
        title: Some(i.title),
        text: Some(place),
        thumbnail_url: i.user.and_then(|u| u.avatar_url),
        facts: vec![
            Fact::Text(state.into()),
            Fact::Count(i.comments, "comments"),
        ],
        live: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(u: &str) -> Url {
        Url::parse(u).unwrap()
    }

    fn what(u: &str) -> Option<String> {
        match GitHub.embed(&url(u), None)? {
            Shown::Card(c) => c.subtitle,
            Shown::Gif(_) => None,
        }
    }

    #[test]
    fn recognises_repositories_issues_and_pull_requests() {
        let base = "https://github.com/nickmorell/bifrost";
        assert_eq!(what(base).as_deref(), Some("Repository"));
        assert_eq!(what(&format!("{base}.git")).as_deref(), Some("Repository"));
        assert_eq!(what(&format!("{base}/issues/5")).as_deref(), Some("Issue"));
        assert_eq!(
            what(&format!("{base}/pull/9/files")).as_deref(),
            Some("Pull request")
        );
        assert_eq!(what(&format!("{base}/issues")), None);
        assert_eq!(what(&format!("{base}/blob/main/README.md")), None);
        assert_eq!(what("https://github.com/nickmorell"), None);
        assert_eq!(what("https://github.com/settings/profile"), None);
        assert_eq!(what("https://github.com/-bad/repo"), None);
        assert_eq!(what("https://gist.github.com/a/b"), None);
    }

    #[test]
    fn asks_the_api_for_the_thing_linked() {
        let l = GitHub
            .lookup(&url("https://github.com/a/b/pull/9"))
            .unwrap();
        assert_eq!(l.url, "https://api.github.com/repos/a/b/pulls/9");
        assert_eq!(l.keep, HOUR);
        let l = GitHub.lookup(&url("https://github.com/a/b.git")).unwrap();
        assert_eq!(l.url, "https://api.github.com/repos/a/b");
        assert_eq!(l.keep, DAY);
    }

    #[test]
    fn reads_a_repository() {
        let body = br#"{"full_name":"a/b","description":"A client","stargazers_count":1234,
            "language":"Rust","archived":true,"owner":{"avatar_url":"https://x/a.png"}}"#;
        let d = GitHub
            .details(&url("https://github.com/a/b"), body)
            .unwrap();
        assert_eq!(d.title.as_deref(), Some("a/b"));
        assert_eq!(d.text.as_deref(), Some("A client"));
        assert_eq!(d.thumbnail_url.as_deref(), Some("https://x/a.png"));
        assert_eq!(
            d.facts,
            [
                Fact::Count(1234, "stars"),
                Fact::Text("Rust".into()),
                Fact::Text("Archived".into())
            ]
        );
        assert_eq!(
            GitHub.details(
                &url("https://github.com/a/b"),
                b"{\"message\":\"Not Found\"}"
            ),
            None
        );
    }

    #[test]
    fn reads_the_state_of_issues_and_pull_requests() {
        let state = |u: &str, body: &str| {
            GitHub
                .details(&url(u), body.as_bytes())
                .map(|d| (d.facts[0].clone(), d.text))
        };
        let issue = "https://github.com/a/b/issues/5";
        let pull = "https://github.com/a/b/pull/9";
        assert_eq!(
            state(issue, r#"{"title":"T","state":"open","comments":2}"#),
            Some((Fact::Text("Open".into()), Some("a/b #5".into())))
        );
        assert_eq!(
            state(issue, r#"{"title":"T","state":"closed"}"#).map(|s| s.0),
            Some(Fact::Text("Closed".into()))
        );
        assert_eq!(
            state(pull, r#"{"title":"T","state":"closed","merged":true}"#).map(|s| s.0),
            Some(Fact::Text("Merged".into()))
        );
        assert_eq!(
            state(pull, r#"{"title":"T","state":"open","draft":true}"#).map(|s| s.0),
            Some(Fact::Text("Draft".into()))
        );
    }
}
