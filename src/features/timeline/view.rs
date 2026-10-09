//! Drawing a timeline: only the posts near the scroll position are drawn
//! ([`lazy_list`]). Posts
//! are drawn by the app, so this builds the app's own message type `M` through
//! [`View::lift`].

use super::key::Key;
use super::merge::item_key;
use super::message::Msg;
use super::place::ESTIMATED_POST;
use super::state::Timelines;
use crate::components::{self as ui, Window, lazy_list, row_key, status};
use crate::model::{Author, FeedItem, FeedReason, FeedReplyContext, MaybePost, PostView};
use crate::theme;
use iced::Element;
use iced::widget::{Column, container};

/// What a timeline needs from the app around it.
pub trait Host<'a, M> {
    /// A post with its actions, drawn as `shape` says.
    fn post(&self, post: &'a PostView, shape: Shape<'a>) -> Element<'a, M>;
    /// Whether moderation hides this post, so a reply is not shown under it.
    fn hidden(&self, post: &PostView) -> bool;
    /// Why the timeline failed to load, with a way to try again.
    fn retry(&self, why: String) -> Element<'a, M>;
}

/// How a post sits in its timeline.
#[derive(Debug, Clone, Copy, Default)]
pub struct Shape<'a> {
    /// Why it is in the timeline (a repost, a pin).
    pub reason: Option<&'a FeedReason>,
    /// Who it answers, when the post it answers is not shown above it.
    pub reply_parent: Option<&'a Author>,
    /// A thread line joins it to the post above.
    pub line_above: bool,
    /// A thread line joins it to the post below.
    pub line_below: bool,
    /// It is only here as the post a reply below it answers.
    pub above_reply: bool,
}

/// A timeline.
pub struct View<'a, H, M> {
    pub timelines: &'a Timelines,
    /// The timeline to draw; `None` while it is not known yet.
    pub key: Option<Key>,
    /// The post picked with the keyboard.
    pub selected: Option<usize>,
    /// The part of the timeline on screen; only posts near it are drawn.
    pub window: Window,
    /// The id of the post at an index, to scroll to it.
    pub post_id: fn(usize) -> iced::widget::Id,
    pub host: H,
    /// Turns the timeline's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<'a, M>, M: 'a> View<'a, H, M> {
    pub fn view(&self) -> Element<'a, M> {
        let (Some(key), Some(tl)) = (
            self.key.as_ref(),
            self.key.as_ref().and_then(|k| self.timelines.get(k)),
        ) else {
            return status("Loading…");
        };
        let host = &self.host;
        let rows = tl.items.iter().map(|item| {
            let id = item_key(item);
            (row_key(id.as_str()), (id, item))
        });
        let draw = |i: usize, (id, item): (String, &'a FeedItem)| {
            // A reply is shown under the post it answers, joined by a thread line, unless
            // that post is the one just above. Replies whose parent is hidden or missing
            // are shown as normal posts.
            let parent = answers(item);
            let just_above = i
                .checked_sub(1)
                .and_then(|j| tl.items.get(j))
                .zip(parent)
                .is_some_and(|(above, p)| above.post.uri == p.uri);
            let mut entry = Column::new();
            if tl.marker() == Some(id.as_str()) {
                entry = entry.push(ui::marker("You left off here"));
            } else if i > 0 && !just_above {
                // Edge to edge, like the hover wash, so the two line up.
                entry = entry.push(ui::divider_flush());
            }
            // The next post answers this one: join them with a line.
            let answered_below = tl
                .items
                .get(i + 1)
                .and_then(answers)
                .is_some_and(|p| p.uri == item.post.uri);
            let shown_parent = parent.filter(|p| !just_above && !host.hidden(p));
            let mut reason = item.reason.as_ref();
            let mut posts = Column::new();
            if let Some(p) = shown_parent {
                posts = posts.push(host.post(
                    p,
                    Shape {
                        reason: reason.take(),
                        line_below: true,
                        above_reply: true,
                        ..Shape::default()
                    },
                ));
            }
            let selected = self.selected == Some(i);
            posts = posts.push(
                host.post(
                    &item.post,
                    Shape {
                        reason,
                        reply_parent: parent
                            .filter(|_| shown_parent.is_none() && !just_above)
                            .map(|p| &p.author),
                        line_above: shown_parent.is_some() || just_above,
                        line_below: answered_below,
                        above_reply: false,
                    },
                ),
            );
            entry
                .push(
                    container(posts)
                        .id((self.post_id)(i))
                        .style(move |t| theme::selected(t, selected)),
                )
                .into()
        };
        let (k, to_app) = (key.clone(), self.lift);
        let measured = move |row, height| to_app(Msg::Measured(k.clone(), row, height));
        let mut list = Column::new().push(lazy_list(
            rows,
            &tl.heights,
            ESTIMATED_POST,
            self.window,
            draw,
            measured,
        ));
        let end = if let Some(e) = &tl.error {
            Some(host.retry(e.clone()))
        } else if tl.loading {
            Some(status("Loading…"))
        } else if tl.items.is_empty() {
            Some(status("Nothing here yet."))
        } else if tl.done {
            Some(status("You're all caught up."))
        } else {
            None
        };
        if let Some(end) = end {
            list = list.push(end);
        }
        list.into()
    }
}

/// The post a timeline entry answers, when it is there to show.
fn answers(item: &FeedItem) -> Option<&PostView> {
    match &item.reply {
        Some(FeedReplyContext {
            parent: MaybePost::Post(p),
        }) => Some(p),
        _ => None,
    }
}
