//! The row of actions under a post: reply, repost or quote, like, bookmark and more.

use super::super::button::action;
use super::super::feedback::tip;
use super::super::icon::Icon;
use super::super::popover::popover;
use crate::theme::{Tone, gap};
use iced::widget::row;
use iced::{Alignment, Element};

/// What the action bar shows and sends. Counts are already shortened ("1.2K") and
/// left out when zero.
pub struct Actions<'a, M> {
    pub replies: Option<String>,
    /// `None` when the author has turned replies off.
    pub reply: Option<M>,
    pub reposts: Option<String>,
    pub reposted: bool,
    /// Opens or closes the repost menu.
    pub repost_menu: M,
    /// The repost menu, while it is open.
    pub repost_popup: Option<Element<'a, M>>,
    pub likes: Option<String>,
    pub liked: bool,
    pub like: M,
    pub bookmarked: bool,
    pub bookmark: M,
    /// Opens or closes the "…" menu.
    pub more: M,
    /// The "…" menu, while it is open.
    pub more_popup: Option<Element<'a, M>>,
    /// Closes whichever menu is open.
    pub close: M,
}

pub fn actions<'a, M: Clone + 'a>(a: Actions<'a, M>) -> Element<'a, M> {
    let reply_label = if a.reply.is_some() {
        "Reply"
    } else {
        "Replies are off"
    };
    row![
        tip(
            action(Icon::Reply, a.replies, false, Tone::Accent, a.reply),
            reply_label,
        ),
        popover(
            tip(
                action(
                    Icon::Repost,
                    a.reposts,
                    a.reposted,
                    Tone::Accent,
                    Some(a.repost_menu),
                ),
                "Repost or quote",
            ),
            a.repost_popup,
            a.close.clone(),
        ),
        tip(
            action(
                if a.liked {
                    Icon::HeartFilled
                } else {
                    Icon::Heart
                },
                a.likes,
                a.liked,
                Tone::Rose,
                Some(a.like),
            ),
            if a.liked { "Unlike" } else { "Like" },
        ),
        iced::widget::space::horizontal(),
        tip(
            action(
                if a.bookmarked {
                    Icon::BookmarkFilled
                } else {
                    Icon::Bookmark
                },
                None,
                a.bookmarked,
                Tone::Accent,
                Some(a.bookmark),
            ),
            if a.bookmarked {
                "Remove bookmark"
            } else {
                "Bookmark"
            },
        ),
        popover(
            tip(
                action(Icon::More, None, false, Tone::Text, Some(a.more)),
                "More",
            ),
            a.more_popup,
            a.close,
        ),
    ]
    .spacing(gap::XS)
    .align_y(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Reply,
        RepostMenu,
        Like,
        Bookmark,
        More,
        Close,
    }

    fn bar(reply: Option<Msg>) -> Element<'static, Msg> {
        actions(Actions {
            replies: Some("3".into()),
            reply,
            reposts: Some("1.2K".into()),
            reposted: true,
            repost_menu: Msg::RepostMenu,
            repost_popup: None,
            likes: Some("40".into()),
            liked: false,
            like: Msg::Like,
            bookmarked: false,
            bookmark: Msg::Bookmark,
            more: Msg::More,
            more_popup: None,
            close: Msg::Close,
        })
    }

    #[test]
    fn counts_send_their_action() {
        for (label, want) in [
            ("3", Msg::Reply),
            ("1.2K", Msg::RepostMenu),
            ("40", Msg::Like),
        ] {
            let mut ui = simulator(bar(Some(Msg::Reply)));
            assert!(ui.click(label).is_ok(), "no {label:?}");
            assert_eq!(ui.into_messages().collect::<Vec<_>>(), [want]);
        }
    }

    #[test]
    fn replies_off_cannot_be_pressed() {
        let mut ui = simulator(bar(None));
        assert!(ui.click("3").is_ok());
        assert!(ui.into_messages().next().is_none());
    }
}
