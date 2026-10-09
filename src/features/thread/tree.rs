//! Walking a thread: a post, the posts above it and the replies below.

use crate::model::{PostView, ThreadNode, ThreadPost};

/// Applies `f` to the post, the posts above it and every reply.
pub(super) fn walk_mut(node: &mut ThreadNode, f: &mut dyn FnMut(&mut PostView)) {
    if let ThreadNode::Post(t) = node {
        f(&mut t.post);
        if let Some(parent) = &mut t.parent {
            walk_mut(parent, f);
        }
        for r in &mut t.replies {
            walk_mut(r, f);
        }
    }
}

pub(super) fn find<'a>(node: &'a ThreadNode, uri: &str) -> Option<&'a PostView> {
    let ThreadNode::Post(t) = node else {
        return None;
    };
    if t.post.uri == uri {
        return Some(&t.post);
    }
    t.parent
        .as_ref()
        .and_then(|p| find(p, uri))
        .or_else(|| t.replies.iter().find_map(|r| find(r, uri)))
}

/// Takes the post at `uri` out of the thread: above the post the thread is about it
/// becomes unavailable; among replies its own replies take its place.
pub(super) fn remove(node: &mut ThreadNode, uri: &str) {
    let ThreadNode::Post(t) = node else {
        return;
    };
    if let Some(parent) = t.parent.as_mut() {
        mark_gone(parent, uri);
    }
    remove_reply(&mut t.replies, uri);
}

/// The ancestor at `uri` becomes unavailable, which also hides the posts above it.
fn mark_gone(node: &mut ThreadNode, uri: &str) {
    let gone = matches!(node, ThreadNode::Post(p) if p.post.uri == uri);
    if gone {
        *node = ThreadNode::NotFound;
    } else if let ThreadNode::Post(p) = node
        && let Some(parent) = p.parent.as_mut()
    {
        mark_gone(parent, uri);
    }
}

fn remove_reply(replies: &mut Vec<ThreadNode>, uri: &str) {
    let mut i = 0;
    while i < replies.len() {
        let hit = matches!(&replies[i], ThreadNode::Post(r) if r.post.uri == uri);
        if hit {
            if let ThreadNode::Post(r) = replies.remove(i) {
                let below = r.replies;
                let n = below.len();
                replies.splice(i..i, below);
                i += n;
            }
        } else {
            if let ThreadNode::Post(r) = &mut replies[i] {
                remove_reply(&mut r.replies, uri);
            }
            i += 1;
        }
    }
}

/// The post, the posts above it, then every reply, in that order.
pub(super) fn collect<'a>(node: &'a ThreadNode, out: &mut Vec<&'a PostView>) {
    if let ThreadNode::Post(t) = node {
        out.push(&t.post);
        if let Some(p) = &t.parent {
            collect(p, out);
        }
        for r in &t.replies {
            collect(r, out);
        }
    }
}

/// The posts above `focus`, nearest first, ending at the first post or at one that
/// is unavailable.
pub(super) fn ancestors(focus: &ThreadPost) -> Vec<&ThreadNode> {
    let mut out = Vec::new();
    let mut cur = focus.parent.as_ref();
    while let Some(n) = cur {
        out.push(n);
        cur = match n {
            ThreadNode::Post(t) => t.parent.as_ref(),
            _ => None,
        };
    }
    out
}

/// Puts the replies to the post a thread was opened on, and theirs, in reading
/// order, once as the thread loads.
pub(super) fn sort(node: &mut ThreadNode, me: &str) {
    let ThreadNode::Post(focus) = node else {
        return;
    };
    let op = match ancestors(focus).last() {
        Some(ThreadNode::Post(t)) => t.post.author.did.to_string(),
        _ => focus.post.author.did.to_string(),
    };
    sort_replies(&mut focus.replies, &op, me);
}

/// Replies in reading order: the original poster's own replies first, then yours
/// (`me`), then people you follow, then the most liked. Unavailable replies go last.
pub(super) fn sort_replies(replies: &mut [ThreadNode], op: &str, me: &str) {
    replies.sort_by_key(|n| match n {
        ThreadNode::Post(t) => {
            let a = &t.post.author;
            let rank = if a.did == op {
                0
            } else if a.did == me {
                1
            } else if a.viewer.following.is_some() {
                2
            } else {
                3
            };
            (rank, std::cmp::Reverse(t.post.like_count))
        }
        _ => (4, std::cmp::Reverse(0)),
    });
    for r in replies {
        if let ThreadNode::Post(t) = r {
            sort_replies(&mut t.replies, op, me);
        }
    }
}
