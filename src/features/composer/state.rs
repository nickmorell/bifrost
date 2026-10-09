//! What the composer holds: the text, what the post replies to or quotes, its
//! images or video, link card, labels, language and who can interact.

use crate::api::richtext::{self, MAX_GRAPHEMES};
use crate::api::{DraftVideo, LinkCard};
use crate::media::image::Prepared;
use crate::model::{AtUri, Author, Interaction, PostView};
use iced::widget::image;
use iced::widget::text_editor::{Action, Content, Edit, Motion};
use std::sync::Arc;

/// Bluesky's limit on images per post in the lexicon BiFrost builds against.
pub(super) const MAX_IMAGES: usize = 4;

/// An image attached to the post, with its preview and alt text.
#[derive(Debug, Clone)]
pub(super) struct Attached {
    pub(super) image: Prepared,
    pub(super) preview: image::Handle,
    pub(super) alt: String,
    /// Which post of a thread it goes on, counting from 0.
    pub(super) part: usize,
}

impl Attached {
    pub(super) fn new(image: Prepared, alt: String) -> Self {
        let preview = image::Handle::from_bytes(image.bytes.clone());
        Self {
            image,
            preview,
            alt,
            part: 0,
        }
    }
}

/// The post as it would be kept in Drafts.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub text: String,
    pub reply_to: Option<AtUri>,
    pub quote: Option<AtUri>,
    pub lang: String,
    /// Image bytes, alt text, width, height.
    pub images: Vec<(Vec<u8>, String, u32, u32)>,
}

/// A post being written.
pub struct Composer {
    pub(super) content: Content,
    pub(super) reply_to: Option<PostView>,
    pub(super) quote: Option<PostView>,
    pub(super) images: Vec<Attached>,
    /// A video instead of images; a post can't have both.
    pub(super) video: Option<DraftVideo>,
    pub(super) suggestions: Vec<Author>,
    /// The `#partial` being typed, for hashtag suggestions.
    pub(super) tag_partial: Option<String>,
    pub(super) emoji_open: bool,
    pub(super) warnings_open: bool,
    /// Content warnings (self-labels) for the attached images.
    pub(super) labels: Vec<&'static str>,
    pub(super) interaction: Interaction,
    pub(super) interaction_open: bool,
    /// Long text is sent as a thread of numbered posts.
    pub(super) thread: bool,
    /// "Don't split" was pressed for text with a break line in it.
    pub(super) no_split: bool,
    pub(super) schedule_open: bool,
    /// A time typed in the Schedule menu.
    pub(super) schedule_text: String,
    /// URL of the card being shown or fetched, and the card once loaded.
    pub(super) card_url: Option<String>,
    pub(super) card: Option<LinkCard>,
    pub(super) card_dismissed: Option<String>,
    pub(super) lang: String,
    pub(super) posting: bool,
    pub(super) error: Option<String>,
    /// Cancel was pressed with something written: asking whether to keep it.
    pub(super) confirm_close: bool,
    /// Posting an image without alt text asks first.
    pub(super) alt_reminder: bool,
    /// Post was pressed with images missing alt text: asking whether to post anyway.
    pub(super) confirm_alt: bool,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            content: Content::new(),
            reply_to: None,
            quote: None,
            images: Vec::new(),
            video: None,
            suggestions: Vec::new(),
            tag_partial: None,
            emoji_open: false,
            warnings_open: false,
            labels: Vec::new(),
            interaction: Interaction::default(),
            interaction_open: false,
            thread: false,
            no_split: false,
            schedule_open: false,
            schedule_text: String::new(),
            card_url: None,
            card: None,
            card_dismissed: None,
            lang: "en".into(),
            posting: false,
            error: None,
            confirm_close: false,
            alt_reminder: true,
            confirm_alt: false,
        }
    }
}

impl Composer {
    /// Tags the post with this language instead of the system one.
    #[must_use]
    pub fn with_lang(mut self, lang: String) -> Self {
        self.lang = lang;
        self
    }

    /// Whether posting an image without alt text asks first.
    #[must_use]
    pub fn with_alt_reminder(mut self, on: bool) -> Self {
        self.alt_reminder = on;
        self
    }

    /// How many images (or the video) have no alt text.
    pub(super) fn missing_alt(&self) -> usize {
        let images = self
            .images
            .iter()
            .filter(|a| a.alt.trim().is_empty())
            .count();
        let video = self.video.as_ref().is_some_and(|v| v.alt.trim().is_empty());
        images + usize::from(video)
    }

    /// Who can reply and quote, until changed.
    #[must_use]
    pub fn with_interaction(mut self, interaction: Interaction) -> Self {
        self.interaction = interaction;
        self
    }

    /// Starts the post with `text`, cursor at the end.
    #[must_use]
    pub fn with_text(mut self, text: &str) -> Self {
        self.content = Content::with_text(text);
        self.content.perform(Action::Move(Motion::DocumentEnd));
        self
    }

    pub fn reply(to: PostView) -> Self {
        Self {
            reply_to: Some(to),
            ..Self::default()
        }
    }

    pub fn quote(post: PostView) -> Self {
        Self {
            quote: Some(post),
            ..Self::default()
        }
    }

    /// Reopens a post kept in Drafts. `reply_to` and `quote` are the posts it referred
    /// to, fetched again; a deleted one is dropped.
    pub fn reopen(saved: Saved, reply_to: Option<PostView>, quote: Option<PostView>) -> Self {
        Self {
            content: Content::with_text(&saved.text),
            reply_to,
            quote,
            images: saved
                .images
                .into_iter()
                .map(|(bytes, alt, width, height)| {
                    Attached::new(
                        Prepared {
                            bytes,
                            width,
                            height,
                        },
                        alt,
                    )
                })
                .collect(),
            lang: if saved.lang.is_empty() {
                "en".into()
            } else {
                saved.lang
            },
            ..Self::default()
        }
    }

    /// Nothing written and no images or video: closing loses nothing.
    pub fn is_empty(&self) -> bool {
        self.text().trim().is_empty() && !self.has_media()
    }

    pub(super) fn has_media(&self) -> bool {
        !self.images.is_empty() || self.video.is_some()
    }

    pub fn is_confirming_close(&self) -> bool {
        self.confirm_close
    }

    pub fn is_posting(&self) -> bool {
        self.posting
    }

    /// The post as it would be kept in Drafts.
    pub fn saved(&self) -> Saved {
        Saved {
            text: self.text(),
            reply_to: self.reply_to.as_ref().map(|p| p.uri.clone()),
            quote: self.quote.as_ref().map(|p| p.uri.clone()),
            lang: self.lang.clone(),
            images: self
                .images
                .iter()
                .map(|a| {
                    (
                        a.image.bytes.clone(),
                        a.alt.clone(),
                        a.image.width,
                        a.image.height,
                    )
                })
                .collect(),
        }
    }

    pub fn reply_parent_uri(&self) -> Option<AtUri> {
        self.reply_to.as_ref().map(|p| p.uri.clone())
    }

    /// Posting failed: say why and let it be tried again.
    pub(super) fn failed(&mut self, error: &str) {
        self.posting = false;
        self.error = Some(format!("Could not post: {error}"));
    }

    pub(super) fn text(&self) -> String {
        let t = self.content.text();
        // text_editor always ends with a newline; it is not part of the post.
        t.strip_suffix('\n').map(str::to_string).unwrap_or(t)
    }

    /// The word just before the cursor, wherever the cursor is in the text.
    fn word_at_cursor(&self) -> Option<String> {
        let cursor = self.content.cursor();
        if cursor.selection.is_some() {
            return None;
        }
        let line = self.content.line(cursor.position.line)?;
        let before = line.text.get(..cursor.position.column)?;
        let word = before.rsplit(char::is_whitespace).next()?;
        (!word.is_empty()).then(|| word.to_string())
    }

    /// The `@partial` being typed, for mention suggestions.
    pub(super) fn mention_partial(&self) -> Option<String> {
        let word = self.word_at_cursor()?;
        richtext::trailing_mention(&word).map(str::to_string)
    }

    pub(super) fn tag_at_cursor(&self) -> Option<String> {
        let word = self.word_at_cursor()?;
        let tag = word.strip_prefix('#')?;
        (!tag.is_empty() && tag.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .then(|| tag.to_string())
    }

    /// Replaces the word just before the cursor, keeping the rest of the text.
    pub(super) fn replace_word(&mut self, with: &str) {
        let Some(word) = self.word_at_cursor() else {
            return;
        };
        for _ in 0..word.chars().count() {
            self.content.perform(Action::Select(Motion::Left));
        }
        self.content
            .perform(Action::Edit(Edit::Paste(Arc::new(with.to_string()))));
    }

    pub(super) fn can_post(&self, text: &str) -> bool {
        if self.threaded() {
            return !super::split::split(text).is_empty();
        }
        let len = richtext::grapheme_len(text);
        len <= MAX_GRAPHEMES && (len > 0 || self.has_media() || self.quote.is_some())
    }
}
