//! Long-form posts on the AT Protocol: Standard.site articles (Leaflet, pckt,
//! Offprint and other blogs publish them) and WhiteWind entries.

use super::AtUri;

/// A Standard.site article's record collection.
pub const DOCUMENT: &str = "site.standard.document";
/// A Standard.site publication (a blog) record collection.
pub const PUBLICATION: &str = "site.standard.publication";
/// Your subscriptions to publications.
pub const SUBSCRIPTION: &str = "site.standard.graph.subscription";
/// A WhiteWind blog entry.
pub const WHITEWIND: &str = "com.whtwnd.blog.entry";

/// An article to open in the reader.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArticleRef {
    /// A Standard.site document record, and the web address it was linked by, which
    /// finds it again if the record has moved and opens it in the browser if not.
    Standard { uri: AtUri, link: Option<String> },
    /// A WhiteWind entry: its author (a handle or DID) and record key.
    WhiteWind { author: String, rkey: String },
}

impl ArticleRef {
    /// Its web address, when known.
    pub fn link(&self) -> Option<&str> {
        match self {
            Self::Standard { link, .. } => link.as_deref(),
            Self::WhiteWind { .. } => None,
        }
    }

    /// The article a WhiteWind address points at:
    /// `https://whtwnd.com/<author>/<rkey>` or `.../<author>/entries/<title>?rkey=<rkey>`.
    pub fn whitewind(url: &str) -> Option<Self> {
        let rest = url
            .strip_prefix("https://whtwnd.com/")
            .or_else(|| url.strip_prefix("https://www.whtwnd.com/"))?;
        let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let from_query = query
            .split('&')
            .find_map(|kv| kv.strip_prefix("rkey="))
            .filter(|k| !k.is_empty());
        let (author, rkey) = match (parts.as_slice(), from_query) {
            ([author, "entries", _], Some(rkey)) => (*author, rkey),
            ([author, rkey], _) if *rkey != "entries" => (*author, *rkey),
            _ => return None,
        };
        let plain = |s: &str| {
            s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_:~".contains(&b))
        };
        (plain(author) && plain(rkey)).then(|| Self::WhiteWind {
            author: author.to_string(),
            rkey: rkey.to_string(),
        })
    }
}

/// An article as the reader shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Article {
    pub title: String,
    pub description: String,
    /// When it was published, as the record gives it (RFC 3339).
    pub published: String,
    /// Its address on the web, to open it in the browser.
    pub url: Option<String>,
    pub cover: Option<String>,
    /// The text, in blocks: headings, paragraphs and list items.
    pub blocks: Vec<Block>,
    pub publication: Option<Publication>,
}

impl Article {
    /// Minutes to read, at 200 words a minute.
    pub fn reading_minutes(&self) -> u32 {
        let words: usize = self
            .blocks
            .iter()
            .map(|b| b.text().split_whitespace().count())
            .sum();
        (words.div_ceil(200)).max(1) as u32
    }
}

/// One block of an article's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(String),
    Paragraph(String),
    Item(String),
    /// Preformatted text, such as code.
    Code(String),
}

impl Block {
    pub fn text(&self) -> &str {
        match self {
            Block::Heading(t) | Block::Paragraph(t) | Block::Item(t) | Block::Code(t) => t,
        }
    }
}

/// Splits plain or Markdown text into blocks: `#` headings, `-`/`*` list items,
/// fenced code, and paragraphs separated by blank lines. Emphasis and link markup
/// are kept as written.
pub fn blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut para: Vec<&str> = Vec::new();
    let mut code: Option<Vec<&str>> = None;
    let flush = |para: &mut Vec<&str>, out: &mut Vec<Block>| {
        if !para.is_empty() {
            out.push(Block::Paragraph(para.join(" ")));
            para.clear();
        }
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if let Some(lines) = code.take() {
                out.push(Block::Code(lines.join("\n")));
            } else {
                flush(&mut para, &mut out);
                code = Some(Vec::new());
            }
            continue;
        }
        if let Some(lines) = &mut code {
            lines.push(line);
            continue;
        }
        if trimmed.is_empty() {
            flush(&mut para, &mut out);
        } else if let Some(h) = heading(trimmed) {
            flush(&mut para, &mut out);
            out.push(Block::Heading(h.to_string()));
        } else if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush(&mut para, &mut out);
            out.push(Block::Item(item.trim().to_string()));
        } else {
            para.push(trimmed);
        }
    }
    if let Some(lines) = code {
        out.push(Block::Code(lines.join("\n")));
    }
    flush(&mut para, &mut out);
    out
}

fn heading(line: &str) -> Option<&str> {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    (1..=6)
        .contains(&hashes)
        .then(|| line[hashes..].strip_prefix(' '))
        .flatten()
        .map(str::trim)
}

/// A publication: a blog that articles belong to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub uri: AtUri,
    pub name: String,
    /// Its home page.
    pub url: String,
    pub icon: Option<String>,
}

/// An article in the Subscriptions list: enough to show and open it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleSummary {
    pub uri: AtUri,
    pub title: String,
    pub description: String,
    pub published: String,
    pub publication: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitewind_addresses_name_their_entry() {
        let w = |author: &str, rkey: &str| {
            Some(ArticleRef::WhiteWind {
                author: author.into(),
                rkey: rkey.into(),
            })
        };
        assert_eq!(
            ArticleRef::whitewind("https://whtwnd.com/alice.test/3kabc"),
            w("alice.test", "3kabc")
        );
        assert_eq!(
            ArticleRef::whitewind("https://whtwnd.com/did:plc:x/entries/My%20post?rkey=3kxyz"),
            w("did:plc:x", "3kxyz")
        );
        assert_eq!(ArticleRef::whitewind("https://whtwnd.com/alice.test"), None);
        assert_eq!(ArticleRef::whitewind("https://example.com/a/b"), None);
        assert_eq!(ArticleRef::whitewind("https://whtwnd.com/a/<b>"), None);
    }

    #[test]
    fn text_splits_into_blocks() {
        let text = "# Title\n\nFirst line\nsame paragraph.\n\n- one\n* two\n```\nlet x = 1;\n\nx\n```\nLast";
        assert_eq!(
            blocks(text),
            [
                Block::Heading("Title".into()),
                Block::Paragraph("First line same paragraph.".into()),
                Block::Item("one".into()),
                Block::Item("two".into()),
                Block::Code("let x = 1;\n\nx".into()),
                Block::Paragraph("Last".into()),
            ]
        );
        assert_eq!(
            blocks("#hashtag start"),
            [Block::Paragraph("#hashtag start".into())]
        );
    }

    #[test]
    fn reading_time_rounds_up() {
        let article = |words: usize| Article {
            title: String::new(),
            description: String::new(),
            published: String::new(),
            url: None,
            cover: None,
            blocks: vec![Block::Paragraph(vec!["word"; words].join(" "))],
            publication: None,
        };
        assert_eq!(article(0).reading_minutes(), 1);
        assert_eq!(article(200).reading_minutes(), 1);
        assert_eq!(article(201).reading_minutes(), 2);
    }
}
