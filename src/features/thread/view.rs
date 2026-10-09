//! The thread screen. Its posts are drawn by the app, so it builds the app's own
//! message type `M` through [`View::lift`] instead of returning [`Msg`] elements.

use super::message::Msg;
use super::state::Threads;
use super::tree;
use crate::components::post_card::{GUIDE, placeholder, thread_guide};
use crate::components::{
    self as ui, Icon, Window, divider, ghost, icon, lazy_list, row_key, status,
};
use crate::model::{AtUri, PostView, ThreadNode, ThreadPost};
use crate::theme::icon_size;
use crate::theme::{Tone, gap};
use iced::widget::{Row, column, container, row};
use iced::{Alignment, Element, Length, Padding};

/// Height assumed for a row not drawn yet.
const ESTIMATED_LINE: f32 = 200.0;
/// Reply levels shown before "Continue thread".
const MAX_DEPTH: usize = 3;

/// How a post sits in the thread.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shape {
    /// The post the thread was opened on: larger text, full timestamp.
    pub focused: bool,
    /// Continue a thread line from the post above into this post's avatar.
    pub line_above: bool,
    /// Draw a thread line from this post's avatar down to the next post.
    pub line_below: bool,
}

/// What the thread screen needs from the app around it.
pub trait Host<M> {
    /// A full post with its actions.
    fn post<'a>(&'a self, post: &'a PostView, shape: Shape) -> Element<'a, M>;
    /// Who can reply to the thread `root` starts, if it is limited.
    fn reply_note(&self, root: &PostView) -> Option<String>;
    /// Why the thread failed to load, with "Sign in again" or "Try again".
    fn retry<'a>(&self, why: String) -> Element<'a, M>;
    /// The screen's scrolling body.
    fn scroll<'a>(&self, content: Element<'a, M>) -> Element<'a, M>;
}

/// The thread screen, drawing [`Threads`].
pub struct View<'a, H, M> {
    pub threads: &'a Threads,
    pub host: &'a H,
    /// The part of the screen showing; only rows near it are drawn.
    pub window: Window,
    /// Turns the feature's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// The thread opened on the post at `uri`.
    pub fn screen(&self, uri: &str) -> Element<'a, M> {
        let node = match self.threads.loaded.get(uri) {
            None => return status("Loading…"),
            Some(Err(e)) => return self.host.retry(e.clone()),
            Some(Ok(node)) => node,
        };
        let ThreadNode::Post(focus) = node else {
            return status(
                "This post is unavailable. It may have been deleted, or you may be blocked.",
            );
        };

        let ancestors = tree::ancestors(focus);
        let root = match ancestors.last() {
            Some(ThreadNode::Post(t)) => Some(&t.post),
            Some(_) => None,
            None => Some(&focus.post),
        };
        let mut lines: Vec<Line<'a>> = Vec::new();
        // The posts above, oldest first, joined into one line down to the focused post.
        for (i, a) in ancestors.iter().rev().enumerate() {
            lines.push(Line::Above(i, a));
        }
        lines.push(Line::Focus(focus, !ancestors.is_empty()));
        if let Some(note) = root.and_then(|r| self.host.reply_note(r)) {
            lines.push(Line::Note(note));
        }
        lines.push(Line::Divider);
        // Replies were put in reading order when the thread loaded.
        for (i, r) in focus.replies.iter().enumerate() {
            replies(&mut lines, r, 0, i > 0);
        }
        let rows = lines.into_iter().map(|l| (l.key(), l));
        let to_app = self.lift;
        let list = lazy_list(
            rows,
            &self.threads.heights,
            ESTIMATED_LINE,
            self.window,
            |_, line| self.line(line),
            move |row, height| to_app(Msg::Measured(row, height)),
        );
        self.host.scroll(list)
    }

    fn line(&self, line: Line<'a>) -> Element<'a, M> {
        match line {
            Line::Above(i, ThreadNode::Post(t)) => self.host.post(
                &t.post,
                Shape {
                    line_above: i > 0,
                    line_below: true,
                    ..Shape::default()
                },
            ),
            Line::Above(i, ThreadNode::Blocked) => placeholder("Blocked post", i > 0, true),
            Line::Above(i, _) => placeholder("Deleted post", i > 0, true),
            Line::Focus(focus, above) => self.host.post(
                &focus.post,
                Shape {
                    focused: true,
                    line_above: above,
                    ..Shape::default()
                },
            ),
            Line::Note(note) => container(
                row![icon(Icon::Lock, icon_size::S, Tone::Faint), ui::faint(note)]
                    .spacing(gap::S)
                    .align_y(Alignment::Center),
            )
            .padding([gap::S, gap::XL])
            .into(),
            Line::Divider => divider(),
            Line::Reply {
                node,
                depth,
                divided,
            } => {
                let reply = match node {
                    ThreadNode::Post(t) => self.host.post(
                        &t.post,
                        Shape {
                            line_below: depth < MAX_DEPTH && !t.replies.is_empty(),
                            ..Shape::default()
                        },
                    ),
                    ThreadNode::Blocked => placeholder("Blocked reply", false, false),
                    _ => placeholder("Deleted reply", false, false),
                };
                let reply = guides(depth, reply);
                if divided {
                    column![ui::divider_flush(), reply].into()
                } else {
                    reply
                }
            }
            Line::More { uri, depth } => guides(
                depth,
                container(ghost(
                    "Continue thread",
                    (self.lift)(Msg::Open(uri.clone())),
                ))
                .padding(Padding::ZERO.left(GUIDE + gap::M))
                .into(),
            ),
        }
    }
}

/// One row of the thread screen, drawn only near the scroll position.
enum Line<'a> {
    /// A post above the focused one, oldest first, with its place in that chain.
    Above(usize, &'a ThreadNode),
    /// The post the thread was opened on, and whether posts sit above it.
    Focus(&'a ThreadPost, bool),
    /// Who can reply.
    Note(String),
    Divider,
    /// A reply at `depth` levels in; `divided` puts a line above a top-level reply.
    Reply {
        node: &'a ThreadNode,
        depth: usize,
        divided: bool,
    },
    /// "Continue thread" under a reply whose own replies are too deep to show.
    More {
        uri: &'a AtUri,
        depth: usize,
    },
}

impl Line<'_> {
    fn key(&self) -> u64 {
        let uri = |n: &ThreadNode| match n {
            ThreadNode::Post(t) => t.post.uri.to_string(),
            _ => String::new(),
        };
        match self {
            Line::Above(i, n) => row_key(&("above", *i, uri(n))),
            Line::Focus(f, _) => row_key(&("focus", f.post.uri.as_str())),
            Line::Note(_) => row_key("note"),
            Line::Divider => row_key("divider"),
            Line::Reply { node, depth, .. } => row_key(&("reply", *depth, uri(node))),
            Line::More { uri, .. } => row_key(&("more", uri.as_str())),
        }
    }
}

/// A reply and, up to [`MAX_DEPTH`], its own replies, in reading order.
fn replies<'a>(lines: &mut Vec<Line<'a>>, node: &'a ThreadNode, depth: usize, divided: bool) {
    lines.push(Line::Reply {
        node,
        depth,
        divided,
    });
    let ThreadNode::Post(t) = node else {
        return;
    };
    if depth < MAX_DEPTH {
        for r in &t.replies {
            replies(lines, r, depth + 1, false);
        }
    } else if !t.replies.is_empty() {
        lines.push(Line::More {
            uri: &t.post.uri,
            depth,
        });
    }
}

/// A reply indented `depth` levels, beside a line running down from the avatar it
/// answers.
fn guides<'a, M: 'a>(depth: usize, content: Element<'a, M>) -> Element<'a, M> {
    if depth == 0 {
        return content;
    }
    let mut r = Row::new();
    for _ in 0..depth {
        r = r.push(thread_guide());
    }
    // The post decides the row's height; the guides stretch to match it.
    r.push(container(content).height(Length::Shrink)).into()
}
