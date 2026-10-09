//! Long text sent as a thread: the posts it splits into, and which images go on
//! which post.

use super::split::{BREAK, split};
use super::state::Composer;
use crate::api::{DraftImage, PostDraft};

impl Composer {
    /// Whether the text goes out as a thread: split was asked for, or a line of
    /// [`BREAK`] is in it and "Don't split" wasn't pressed.
    pub(super) fn threaded(&self) -> bool {
        self.thread || (!self.no_split && self.text().lines().any(|l| l.trim() == BREAK))
    }

    /// The posts the text splits into, numbered.
    pub(super) fn parts(&self) -> Vec<String> {
        split(&self.text())
    }

    /// The thread as it would be sent now: the first post carries the reply, quote,
    /// video and link card; each image goes on the post picked for it.
    pub(super) fn thread_drafts(&self) -> Vec<PostDraft> {
        let first = self.draft();
        let parts = split(&first.text);
        let last = parts.len().saturating_sub(1);
        let mut images: Vec<Vec<DraftImage>> = vec![Vec::new(); parts.len()];
        for (a, img) in self.images.iter().zip(&first.images) {
            if let Some(on) = images.get_mut(a.part.min(last)) {
                on.push(img.clone());
            }
        }
        parts
            .into_iter()
            .zip(images)
            .enumerate()
            .map(|(i, (text, images))| {
                let lead = i == 0;
                let video = if lead { first.video.clone() } else { None };
                let media = !images.is_empty() || video.is_some();
                PostDraft {
                    text,
                    reply: if lead { first.reply.clone() } else { None },
                    quote: if lead { first.quote.clone() } else { None },
                    link_card: if lead && !media {
                        self.card.clone()
                    } else {
                        None
                    },
                    labels: if media {
                        self.labels.iter().map(ToString::to_string).collect()
                    } else {
                        Vec::new()
                    },
                    langs: first.langs.clone(),
                    interaction: first.interaction.clone(),
                    images,
                    video,
                }
            })
            .collect()
    }
}
