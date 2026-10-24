//! The links part of "Edit profile": up to 10 links, each an https address and an
//! optional title, in the order the profile shows them.

use crate::api::richtext::grapheme_len;
use crate::api::{LinkEdit, LinksEdit};
use crate::components::{self as ui, Icon, ghost, icon_button};
use crate::model::{AtUri, Profile, StrongRef};
use crate::theme::{self, gap, size};
use iced::widget::{Column, column, row, space, text};
use iced::{Alignment, Length};

/// Bluesky's limits.
pub(super) const MAX_LINKS: usize = 10;
pub(super) const MAX_TITLE: usize = 40;

#[derive(Debug, Clone)]
pub enum LinkMsg {
    Url(usize, String),
    Title(usize, String),
    Add,
    Remove(usize),
    /// Move a link up (`true`) or down one place.
    Move(usize, bool),
}

/// One link in the dialog.
struct Row {
    /// Its record, for a link already on the profile.
    existing: Option<StrongRef>,
    /// Its address and title as loaded, to tell whether they changed.
    loaded: Option<(String, String)>,
    url: String,
    title: String,
}

#[derive(Default)]
pub(super) struct Links {
    rows: Vec<Row>,
    /// Links on the profile that were removed.
    removed: Vec<AtUri>,
    /// Whether anything was changed, so an untouched list is left as it is.
    touched: bool,
}

/// `input` as an https address, adding `https://` when it has no scheme, or `None`
/// when it isn't one.
pub(super) fn web_address(input: &str) -> Option<String> {
    let t = input.trim();
    let full = if t.contains("://") {
        t.to_string()
    } else {
        format!("https://{t}")
    };
    let parsed = url::Url::parse(&full).ok()?;
    let host = parsed.host_str().unwrap_or_default();
    (parsed.scheme() == "https" && host.contains('.')).then_some(full)
}

impl Links {
    pub(super) fn new(p: &Profile) -> Self {
        let rows = p
            .links
            .iter()
            .filter(|l| l.record.is_some())
            .map(|l| {
                let title = l.title.clone().unwrap_or_default();
                Row {
                    existing: l.record.clone(),
                    loaded: Some((l.url.clone(), title.trim().to_string())),
                    url: l.url.clone(),
                    title,
                }
            })
            .collect();
        Self {
            rows,
            ..Self::default()
        }
    }

    pub(super) fn update(&mut self, msg: LinkMsg) {
        self.touched = true;
        let len = self.rows.len();
        match msg {
            LinkMsg::Url(i, v) => {
                if let Some(r) = self.rows.get_mut(i) {
                    r.url = v.trim().to_string();
                }
            }
            LinkMsg::Title(i, v) => {
                if let Some(r) = self.rows.get_mut(i) {
                    r.title = v.replace('\n', " ");
                }
            }
            LinkMsg::Add if len < MAX_LINKS => self.rows.push(Row {
                existing: None,
                loaded: None,
                url: String::new(),
                title: String::new(),
            }),
            LinkMsg::Add => {}
            LinkMsg::Remove(i) if i < len => {
                if let Some(r) = self.rows.remove(i).existing {
                    self.removed.push(r.uri);
                }
            }
            LinkMsg::Remove(_) => {}
            LinkMsg::Move(i, up) => {
                let other = if up { i.checked_sub(1) } else { Some(i + 1) };
                if let Some(j) = other.filter(|&j| i < len && j < len) {
                    self.rows.swap(i, j);
                }
            }
        }
    }

    pub(super) fn valid(&self) -> bool {
        self.rows
            .iter()
            .all(|r| web_address(&r.url).is_some() && grapheme_len(&r.title) <= MAX_TITLE)
    }

    /// The links to save, or `None` when they weren't touched.
    pub(super) fn edit(&self) -> Option<LinksEdit> {
        if !self.touched {
            return None;
        }
        let links = self
            .rows
            .iter()
            .map(|r| {
                let url = web_address(&r.url).unwrap_or_default();
                let title = r.title.trim().to_string();
                let changed = r.loaded.as_ref() != Some(&(url.clone(), title.clone()));
                LinkEdit {
                    existing: r.existing.clone(),
                    url,
                    title,
                    changed,
                }
            })
            .collect();
        Some(LinksEdit {
            links,
            removed: self.removed.clone(),
        })
    }

    /// The section; `lift` turns the dialog's messages into the app's.
    pub(super) fn view<'a, M: Clone + 'a>(&'a self, lift: fn(super::Msg) -> M) -> Column<'a, M> {
        let wrap = move |m| lift(super::Msg::Link(m));
        let mut col = column![row![
            ui::faint("Links"),
            space::horizontal(),
            text(format!("{}/{MAX_LINKS}", self.rows.len()))
                .size(size::CAPTION)
                .style(theme::faint),
        ]]
        .spacing(gap::XS);
        for (i, r) in self.rows.iter().enumerate() {
            col = col.push(
                row![
                    ui::field("example.com", &r.url)
                        .on_input(move |v| wrap(LinkMsg::Url(i, v)))
                        .width(Length::FillPortion(3)),
                    ui::field("Title (optional)", &r.title)
                        .on_input(move |v| wrap(LinkMsg::Title(i, v)))
                        .width(Length::FillPortion(2)),
                    ui::tip(
                        icon_button(Icon::ArrowUp, wrap(LinkMsg::Move(i, true))),
                        "Move up"
                    ),
                    ui::tip(
                        icon_button(Icon::ArrowDown, wrap(LinkMsg::Move(i, false))),
                        "Move down"
                    ),
                    ui::tip(icon_button(Icon::Trash, wrap(LinkMsg::Remove(i))), "Remove"),
                ]
                .spacing(gap::XS)
                .align_y(Alignment::Center),
            );
            let problem = if !r.url.is_empty() && web_address(&r.url).is_none() {
                Some("Links must be https web addresses")
            } else if grapheme_len(&r.title) > MAX_TITLE {
                Some("Titles can be up to 40 characters")
            } else {
                None
            };
            if let Some(p) = problem {
                col = col.push(text(p).size(size::CAPTION).style(theme::danger_text));
            }
        }
        if self.rows.len() < MAX_LINKS {
            col = col.push(ghost("Add link", wrap(LinkMsg::Add)));
        }
        col
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn links() -> Links {
        let p: Profile = serde_json::from_value(json!({
            "did": "did:plc:a", "handle": "a.test",
            "links": [
                {"uri": "at://did:plc:a/app.bsky.actor.link/1", "cid": "c1", "url": "https://one.test", "title": "One"},
                {"uri": "at://did:plc:a/app.bsky.actor.link/2", "cid": "c2", "url": "https://two.test"}
            ]
        }))
        .unwrap();
        Links::new(&p)
    }

    #[test]
    fn addresses_are_https_only() {
        assert_eq!(
            web_address(" blog.test/x "),
            Some("https://blog.test/x".into())
        );
        assert_eq!(web_address("https://a.test"), Some("https://a.test".into()));
        assert_eq!(web_address("http://a.test"), None);
        assert_eq!(web_address("javascript:alert(1)"), None);
        assert_eq!(web_address("localhost"), None);
        assert_eq!(web_address(""), None);
    }

    #[test]
    fn untouched_links_are_left_alone() {
        assert_eq!(links().edit(), None);
    }

    #[test]
    fn edits_say_what_changed_and_what_went() {
        let mut l = links();
        l.update(LinkMsg::Move(1, true));
        l.update(LinkMsg::Title(1, "First".into()));
        l.update(LinkMsg::Add);
        assert!(!l.valid(), "a new link needs an address");
        l.update(LinkMsg::Url(2, "new.test".into()));
        l.update(LinkMsg::Remove(0));
        assert!(l.valid());

        let edit = l.edit().unwrap();
        let summary: Vec<_> = edit
            .links
            .iter()
            .map(|e| {
                (
                    e.url.as_str(),
                    e.title.as_str(),
                    e.changed,
                    e.existing.is_some(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("https://one.test", "First", true, true),
                ("https://new.test", "", true, false)
            ]
        );
        assert_eq!(
            edit.removed[0].as_str(),
            "at://did:plc:a/app.bsky.actor.link/2"
        );
    }

    #[test]
    fn there_are_at_most_ten() {
        let mut l = Links::default();
        for _ in 0..12 {
            l.update(LinkMsg::Add);
        }
        assert_eq!(l.rows.len(), MAX_LINKS);
        l.update(LinkMsg::Move(9, false));
        l.update(LinkMsg::Remove(10));
        assert_eq!(l.rows.len(), MAX_LINKS);
    }
}
