//! Post text and plain text with their mentions, links and tags made clickable.

use crate::api::richtext;
use crate::app::{LinkTarget, Message, nav};
use crate::model::{Facet, FacetFeature, PostRecord};
use crate::theme::{Look, Tone};
use iced::widget::text::Span;
use iced::widget::{rich_text, span};
use iced::{Color, Element};

/// Post text with mentions, links and tags made clickable.
pub fn post_text(look: Look, record: &PostRecord, size: f32) -> Element<'_, Message> {
    let (font, size) = look.reading(size);
    rich_text(spans(record, look.color(Tone::Accent)))
        .font(font)
        .size(size)
        .line_height(1.55)
        .on_link_click(|a| Message::Nav(nav::Msg::Link(a)))
        .into()
}

/// Plain text, such as a profile bio, with its links, mentions and tags made clickable.
pub fn linked_text(look: Look, text: &str, size: f32) -> Element<'_, Message> {
    let (font, size) = look.reading(size);
    let mut out = Vec::new();
    let mut pos = 0;
    for (range, target) in crate::app::util::linkify(text) {
        if range.start > pos {
            out.push(span(&text[pos..range.start]));
        }
        out.push(
            span(&text[range.clone()])
                .color(look.color(Tone::Accent))
                .link(target),
        );
        pos = range.end;
    }
    if pos < text.len() {
        out.push(span(&text[pos..]));
    }
    rich_text(out)
        .font(font)
        .size(size)
        .line_height(1.55)
        .on_link_click(|a| Message::Nav(nav::Msg::Link(a)))
        .into()
}

fn spans(record: &PostRecord, link: Color) -> Vec<Span<'_, LinkTarget>> {
    let text = record.text.as_str();
    let mut facets: Vec<&Facet> = record
        .facets
        .iter()
        .filter(|f| {
            f.index.byte_start < f.index.byte_end
                && f.index.byte_end <= text.len()
                && text.is_char_boundary(f.index.byte_start)
                && text.is_char_boundary(f.index.byte_end)
        })
        .collect();
    facets.sort_by_key(|f| f.index.byte_start);

    let mut out = Vec::new();
    let mut pos = 0;
    for f in facets {
        if f.index.byte_start < pos {
            continue; // overlapping facet: ignore
        }
        if f.index.byte_start > pos {
            plain(&text[pos..f.index.byte_start], link, &mut out);
        }
        let segment = &text[f.index.byte_start..f.index.byte_end];
        let target = f.features.iter().find_map(|feat| match feat {
            FacetFeature::Mention { did } => Some(LinkTarget::Profile(did.to_string())),
            FacetFeature::Link { uri } => Some(match shown_host(segment) {
                Some(shown) if Some(&shown) != host(uri).as_ref() => LinkTarget::Disguised {
                    shown: segment.to_string(),
                    url: uri.clone(),
                },
                _ => LinkTarget::Url(uri.clone()),
            }),
            FacetFeature::Tag { tag } => Some(LinkTarget::Tag(tag.clone())),
            FacetFeature::Other => None,
        });
        out.push(match target {
            Some(t) => span(segment).color(link).link(t),
            None => span(segment),
        });
        pos = f.index.byte_end;
    }
    if pos < text.len() {
        plain(&text[pos..], link, &mut out);
    }
    out
}

/// Text outside facets, with cashtags such as `$AAPL` made clickable: Bluesky's own
/// app doesn't mark them, so they are found here.
fn plain<'a>(text: &'a str, link: Color, out: &mut Vec<Span<'a, LinkTarget>>) {
    let mut pos = 0;
    let mut start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                start = None;
                let word =
                    text[s..i].trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '"', '\'']);
                let Some(ticker) = richtext::cashtag(word) else {
                    continue;
                };
                if s > pos {
                    out.push(span(&text[pos..s]));
                }
                let end = s + word.len();
                out.push(
                    span(&text[s..end])
                        .color(link)
                        .link(LinkTarget::Tag(ticker)),
                );
                pos = end;
            }
            _ => {}
        }
    }
    if pos < text.len() {
        out.push(span(&text[pos..]));
    }
}

/// The site a link's text names, when the text reads like an address ("example.com",
/// "https://example.com/a…"), so a link can't show one site and open another.
fn shown_host(segment: &str) -> Option<String> {
    let t = segment.trim().trim_end_matches(['…', '.']);
    if t.is_empty() || t.contains(char::is_whitespace) {
        return None;
    }
    let full = if t.contains("://") {
        t.to_string()
    } else {
        format!("https://{t}")
    };
    host(&full).filter(|h| h.contains('.'))
}

/// A link's host, lower case and without `www.`.
fn host(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    let h = u.host_str()?.to_ascii_lowercase();
    Some(h.strip_prefix("www.").unwrap_or(&h).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn spans_split_on_facets_and_skip_bad_ones() {
        let rec: PostRecord = serde_json::from_value(json!({
            "text": "hi @a.test and more",
            "facets": [
                {"index": {"byteStart": 3, "byteEnd": 10}, "features": [{"$type": "app.bsky.richtext.facet#mention", "did": "did:plc:a"}]},
                {"index": {"byteStart": 5, "byteEnd": 99}, "features": [{"$type": "app.bsky.richtext.facet#link", "uri": "https://x"}]}
            ]
        }))
        .unwrap();
        let s = spans(&rec, Color::WHITE);
        let texts: Vec<&str> = s.iter().map(|sp| sp.text.as_ref()).collect();
        assert_eq!(texts, vec!["hi ", "@a.test", " and more"]);
        assert_eq!(s[1].link, Some(LinkTarget::Profile("did:plc:a".into())));
    }

    #[test]
    fn cashtags_outside_facets_are_clickable() {
        let rec: PostRecord =
            serde_json::from_value(json!({"text": "Buying $aapl, not $5. $TSLA"})).unwrap();
        let s = spans(&rec, Color::WHITE);
        let texts: Vec<&str> = s.iter().map(|sp| sp.text.as_ref()).collect();
        assert_eq!(texts, vec!["Buying ", "$aapl", ", not $5. ", "$TSLA"]);
        assert_eq!(s[1].link, Some(LinkTarget::Tag("$AAPL".into())));
        assert_eq!(s[2].link, None);
    }

    fn target(text: &str, uri: &str) -> Option<LinkTarget> {
        let rec: PostRecord = serde_json::from_value(json!({
            "text": text,
            "facets": [{"index": {"byteStart": 0, "byteEnd": text.len()},
                "features": [{"$type": "app.bsky.richtext.facet#link", "uri": uri}]}]
        }))
        .unwrap();
        spans(&rec, Color::WHITE)[0].link.clone()
    }

    #[test]
    fn links_showing_another_site_ask_first() {
        let url = |u: &str| Some(LinkTarget::Url(u.into()));
        // Text that names the same site, or no site at all, opens straight away.
        assert_eq!(
            target("example.com/a…", "https://www.example.com/a/long"),
            url("https://www.example.com/a/long")
        );
        assert_eq!(
            target("https://Example.com", "https://example.com/"),
            url("https://example.com/")
        );
        assert_eq!(
            target("read this", "https://x.test/"),
            url("https://x.test/")
        );
        assert_eq!(target("docs", "https://x.test/"), url("https://x.test/"));
        // Text naming one site that opens another doesn't.
        assert_eq!(
            target("bank.com", "https://bank.com.evil.test/login"),
            Some(LinkTarget::Disguised {
                shown: "bank.com".into(),
                url: "https://bank.com.evil.test/login".into()
            })
        );
        assert!(matches!(
            target("https://bsky.app/settings", "https://evil.test"),
            Some(LinkTarget::Disguised { .. })
        ));
    }
}
