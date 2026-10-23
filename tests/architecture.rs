//! Architecture rules, checked on every `cargo test` (see CLAUDE.md).
//!
//! Each top-level module under `src/` is a black box. These tests read the source and
//! fail when a module reaches into one it is not allowed to use, when GUI code leaks
//! out of the UI, or when a file grows past the size limit. Changing a rule here is a
//! design decision: update CLAUDE.md in the same change.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Which other top-level modules each module may use. `app` is the app shell and may
/// use everything; nothing may use `app`.
const ALLOWED: &[(&str, &[&str])] = &[
    ("model", &[]),
    ("platform", &[]),
    ("net", &[]),
    ("theme", &["platform"]),
    ("components", &["theme"]),
    (
        "features",
        &["api", "components", "media", "model", "moderation", "theme"],
    ),
    ("storage", &["model", "platform"]),
    ("auth", &["net", "platform", "storage"]),
    ("api", &["auth", "model", "net", "storage"]),
    ("translate", &["api", "model", "net", "storage"]),
    ("integrations", &["storage"]),
    ("moderation", &["model"]),
    ("video", &["api", "net"]),
    ("media", &["api", "platform"]),
];
/// Modules that may use the GUI toolkit. Everything else stays testable without it.
const GUI: &[&str] = &["app", "components", "features", "theme", "video"];

/// No source file may be longer than this.
const MAX_LINES: usize = 400;

/// Files already over the limit when the rule was introduced, with their size now.
/// They may only shrink; the restructure splits them and removes them from this list.
/// `cargo test` fails until a shrunk file's number here is lowered to match it.
const OVERSIZED: &[(&str, usize)] = &[("src/api/mod.rs", 1010), ("src/video/mod.rs", 515)];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `src/api/mod.rs` -> `src/api/mod.rs`, with forward slashes on every OS.
fn relative(path: &Path) -> String {
    let rel = path.strip_prefix(root()).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The top-level module a file under `src/` belongs to, or `None` for the crate root
/// files and binaries.
fn module_of(rel: &str) -> Option<String> {
    let rest = rel.strip_prefix("src/")?;
    let first = rest.split('/').next()?;
    if first == "bin" || first == "main.rs" || first == "lib.rs" {
        return None;
    }
    Some(first.trim_end_matches(".rs").to_string())
}

/// Code lines of a file, without line comments, numbered from 1.
fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.split("//").next().unwrap_or("")))
}

/// Every `crate::<module>` a line mentions.
fn crate_paths(line: &str) -> Vec<String> {
    line.match_indices("crate::")
        .filter(|(i, _)| *i == 0 || !line.as_bytes()[i - 1].is_ascii_alphanumeric())
        .map(|(i, m)| {
            line[i + m.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        })
        .filter(|m: &String| !m.is_empty())
        .collect()
}

fn sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    rust_files(&root().join("src"), &mut files);
    files.sort();
    files
        .iter()
        .map(|p| (relative(p), fs::read_to_string(p).unwrap_or_default()))
        .collect()
}

#[test]
fn modules_only_use_what_they_are_allowed_to() {
    let allowed: BTreeMap<&str, BTreeSet<&str>> = ALLOWED
        .iter()
        .map(|(m, deps)| (*m, deps.iter().copied().collect()))
        .collect();
    let mut problems = Vec::new();
    for (rel, text) in sources() {
        let Some(module) = module_of(&rel) else {
            continue;
        };
        for (n, line) in code_lines(&text) {
            for used in crate_paths(line) {
                if used == module {
                    continue;
                }
                let ok = module == "app"
                    || (used != "app"
                        && allowed
                            .get(module.as_str())
                            .is_some_and(|d| d.contains(used.as_str())));
                if !ok {
                    problems.push(format!("{rel}:{n}: `{module}` may not use `crate::{used}`"));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "module boundary broken (rules in tests/architecture.rs):\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_module_has_a_rule() {
    let known: BTreeSet<&str> = ALLOWED.iter().map(|(m, _)| *m).chain(["app"]).collect();
    let missing: BTreeSet<String> = sources()
        .iter()
        .filter_map(|(rel, _)| module_of(rel))
        .filter(|m| !known.contains(m.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "new top-level modules need an entry in ALLOWED: {missing:?}"
    );
}

#[test]
fn gui_code_stays_in_gui_modules() {
    let mut problems = Vec::new();
    for (rel, text) in sources() {
        let Some(module) = module_of(&rel) else {
            continue;
        };
        if GUI.contains(&module.as_str()) {
            continue;
        }
        for (n, line) in code_lines(&text) {
            if line.contains("iced::") || line.trim_start().starts_with("use iced") {
                problems.push(format!("{rel}:{n}: `{module}` may not use iced"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "GUI code outside the UI:\n{}",
        problems.join("\n")
    );
}

#[test]
fn files_stay_small() {
    let grandfathered: BTreeMap<&str, usize> = OVERSIZED.iter().copied().collect();
    let mut problems = Vec::new();
    for (rel, text) in sources() {
        let lines = text.lines().count();
        let limit = grandfathered
            .get(rel.as_str())
            .copied()
            .unwrap_or(MAX_LINES);
        if lines > limit {
            problems.push(format!("{rel}: {lines} lines (limit {limit})"));
        }
    }
    // The limits follow the files down, so a file that shrank can't grow back.
    for (rel, limit) in OVERSIZED {
        let lines = fs::read_to_string(root().join(rel)).map_or(0, |t| t.lines().count());
        if lines <= MAX_LINES {
            problems.push(format!(
                "{rel}: now {lines} lines; remove it from OVERSIZED"
            ));
        } else if lines < *limit {
            problems.push(format!(
                "{rel}: now {lines} lines; lower OVERSIZED to {lines}"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "files over the size limit; split them into smaller modules:\n{}",
        problems.join("\n")
    );
}

/// The features named by every `crate::features::` path on a line, including each
/// name in a `crate::features::{..}` group.
fn feature_paths(line: &str) -> Vec<String> {
    const PREFIX: &str = "crate::features::";
    let ident = |s: &str| -> String {
        s.trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect()
    };
    line.match_indices(PREFIX)
        .flat_map(|(i, _)| {
            let rest = &line[i + PREFIX.len()..];
            match rest.strip_prefix('{') {
                Some(group) => group
                    .split('}')
                    .next()
                    .unwrap_or("")
                    .split(',')
                    .map(ident)
                    .collect(),
                None => vec![ident(rest)],
            }
        })
        .filter(|m| !m.is_empty())
        .collect()
}

/// The feature a file under `src/features/` belongs to; `None` for the shared
/// `features/mod.rs`.
fn feature_of(rel: &str) -> Option<&str> {
    let rest = rel.strip_prefix("src/features/")?;
    let (first, _) = rest.split_once('/')?;
    Some(first)
}

#[test]
fn features_never_use_each_other() {
    let mut features = Vec::new();
    rust_files(&root().join("src/features"), &mut features);
    let names: BTreeSet<String> = features
        .iter()
        .filter_map(|p| feature_of(&relative(p)).map(str::to_string))
        .collect();
    let mut problems = Vec::new();
    for (rel, text) in sources() {
        let Some(feature) = feature_of(&rel) else {
            continue;
        };
        for (n, line) in code_lines(&text) {
            for used in feature_paths(line) {
                if used != feature && names.contains(&used) {
                    problems.push(format!(
                        "{rel}:{n}: `{feature}` may not use feature `{used}`"
                    ));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "features use only what features/mod.rs shares, never another feature:\n{}",
        problems.join("\n")
    );
}

/// The theme may read the system font's name from `platform`, and nothing else there.
#[test]
fn theme_uses_only_the_platform_fonts() {
    let mut problems = Vec::new();
    for (rel, text) in sources() {
        if module_of(&rel).as_deref() != Some("theme") {
            continue;
        }
        for (n, line) in code_lines(&text) {
            for (i, _) in line.match_indices("crate::platform") {
                if !line[i..].starts_with("crate::platform::fonts::") {
                    problems.push(format!(
                        "{rel}:{n}: the theme may use only `platform::fonts`"
                    ));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn crate_paths_are_found() {
    assert_eq!(crate_paths("use crate::api::Client;"), ["api"]);
    assert_eq!(
        crate_paths("let x = crate::auth::a(); crate::app::b()"),
        ["auth", "app"]
    );
    assert!(crate_paths("use atrium_xrpc_client::reqwest;").is_empty());
    assert_eq!(module_of("src/api/mod.rs").as_deref(), Some("api"));
    assert_eq!(module_of("src/settings.rs").as_deref(), Some("settings"));
    assert_eq!(module_of("src/main.rs"), None);
    assert_eq!(feature_paths("use crate::features::chat::Msg;"), ["chat"]);
    assert_eq!(
        feature_paths("use crate::features::{Pictures, thread::Threads};"),
        ["Pictures", "thread"]
    );
    assert_eq!(feature_of("src/features/chat/view/list.rs"), Some("chat"));
    assert_eq!(feature_of("src/features/mod.rs"), None);
}
