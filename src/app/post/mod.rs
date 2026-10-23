//! A single post: deciding what it shows (moderation, embeds, menus) and handing
//! that to the `post_card` component, plus its rich text.

mod article;
mod embed;
mod linked;
mod more;

use embed::embed_view;

pub(super) use embed::quoted_post;
pub use linked::{linked_text, post_text};
use more::menu;

use crate::app::actions;
use crate::app::compose;
use crate::app::nav;
use crate::app::popups;

use crate::api::{QuoteSort, parse_at_uri};
use crate::app::util::{avatar, compact, full_time, relative_time};
use crate::app::{App, Menu, Message, Route, Toggle};
use crate::components::post_card as pc;
use crate::components::{self as ui, Icon};
use crate::features::Pictures;
use crate::model::{AtUri, Author, FeedReason, PostView};
use crate::moderation::Context;
use crate::theme::{gap, size};
use iced::widget::{Column, container};
use iced::{Element, Padding};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Normal,
    /// The post a thread is about: larger text, full timestamp.
    Focused,
}

pub struct Opts<'a> {
    pub ctx: Context,
    pub size: Size,
    pub reason: Option<&'a FeedReason>,
    pub reply_parent: Option<&'a Author>,
    /// Draw a thread line from this post's avatar down to the next post.
    pub line_below: bool,
    /// Continue a thread line from the post above into this post's avatar.
    pub line_above: bool,
    /// Shown only as the post a reply below it answers.
    pub above_reply: bool,
}

impl Default for Opts<'_> {
    fn default() -> Self {
        Self {
            ctx: Context::Feed,
            size: Size::Normal,
            reason: None,
            reply_parent: None,
            line_below: false,
            line_above: false,
            above_reply: false,
        }
    }
}

/// A full post with actions.
pub fn view<'a>(app: &'a App, post: &'a PostView, opts: Opts<'a>) -> Element<'a, Message> {
    let decision = app.moderate(post, opts.ctx);
    let revealed = app.revealed.contains(&post.uri);

    if let Some(reason) = &decision.hide
        && !revealed
    {
        return pc::hidden(
            reason.clone(),
            Message::Popup(popups::Msg::Reveal(post.uri.clone())),
        );
    }

    let focused = opts.size == Size::Focused;
    let a = &post.author;
    let mut body = Column::new().spacing(gap::S);
    body = body.push(pc::header(
        pc::Header {
            name: a.name(),
            handle: &a.handle,
            time: (!focused)
                .then(|| (relative_time(&post.indexed_at), full_time(&post.indexed_at))),
            large: focused,
            verified: app.badge(a.verified()),
            width: app.post_width(),
        },
        Message::Nav(nav::Msg::OpenProfile(a.did.to_string())),
    ));

    if let Some(parent) = opts.reply_parent {
        body = body.push(pc::context_line(
            Icon::Reply,
            format!("Replying to {}", parent.name()),
            Message::Nav(nav::Msg::OpenProfile(parent.did.to_string())),
        ));
    }

    let text_size = if focused {
        size::LARGE + 1.0
    } else {
        size::BODY
    };
    if !post.record.text.is_empty() {
        body = body.push(post_text(app.look(), &post.record, text_size));
    }
    if let Some(t) = app.translation_view(post, text_size) {
        body = body.push(t);
    }

    if let Some(embed) = &post.embed {
        body = body.push(
            container(embed_view(app, embed, post, &decision, revealed))
                .padding(Padding::ZERO.top(gap::XS)),
        );
    }

    if focused {
        body = body.push(ui::faint(format_full_time(&post.record.created_at)));
    }

    let (repost_menu, more_menu, context_menu) = if opts.above_reply {
        (Menu::RepostAbove, Menu::MoreAbove, Menu::ContextAbove)
    } else {
        (Menu::Repost, Menu::More, Menu::Context)
    };
    body = body.push(actions(app, post, repost_menu, more_menu));

    let above = match opts.reason {
        Some(FeedReason::Repost { by }) => Some(pc::context_line(
            Icon::Repost,
            format!("Reposted by {}", by.name()),
            Message::Nav(nav::Msg::OpenProfile(by.did.to_string())),
        )),
        Some(FeedReason::Pin) => Some(pc::pinned()),
        _ => None,
    };
    pc::card(pc::Card {
        avatar: author_avatar(app, a),
        body,
        above,
        line_above: opts.line_above,
        line_below: opts.line_below,
        open: (!focused).then(|| Message::Nav(nav::Msg::OpenThread(post.uri.clone()))),
        menu: Some(pc::ContextMenu {
            open: Message::Popup(popups::Msg::ToggleMenu(post.uri.clone(), context_menu)),
            popup: menu_open(app, post, context_menu).then(|| menu(app, post)),
            close: Message::Popup(popups::Msg::CloseMenu),
        }),
    })
}

/// `menu` is open on `post`.
fn menu_open(app: &App, post: &PostView, menu: Menu) -> bool {
    app.popups
        .menu
        .as_ref()
        .is_some_and(|(u, m)| u == &post.uri && *m == menu)
}

fn actions<'a>(
    app: &'a App,
    post: &'a PostView,
    repost_menu: Menu,
    more_menu: Menu,
) -> Element<'a, Message> {
    let open = |menu: Menu| menu_open(app, post, menu);
    let liked = post.viewer.like.is_some();
    let reposted = post.viewer.repost.is_some();
    let bookmarked = post.viewer.bookmarked;
    let count = |n: u64| (n > 0).then(|| compact(n));
    let uri = &post.uri;
    let quote_off = post.viewer.embedding_disabled;
    let toggle = |on: bool| if on { Toggle::Off } else { Toggle::On };
    pc::actions(pc::Actions {
        replies: count(post.reply_count),
        reply: (!post.viewer.reply_disabled)
            .then(|| Message::Compose(compose::Msg::Reply(uri.clone()))),
        reposts: count(post.repost_count + post.quote_count),
        reposted,
        repost_menu: Message::Popup(popups::Msg::ToggleMenu(uri.clone(), repost_menu)),
        repost_popup: open(repost_menu).then(|| {
            let mut items = vec![
                ui::menu_item(
                    Icon::Repost,
                    if reposted { "Undo repost" } else { "Repost" },
                    Some(Message::Action(actions::Msg::Repost(
                        uri.clone(),
                        toggle(reposted),
                    ))),
                ),
                ui::menu_item(
                    Icon::Quote,
                    if quote_off {
                        "Quoting is off"
                    } else {
                        "Quote post"
                    },
                    (!quote_off).then(|| Message::Compose(compose::Msg::Quote(uri.clone()))),
                ),
            ];
            if post.quote_count > 0 {
                items.push(ui::menu_item(
                    Icon::List,
                    match post.quote_count {
                        1 => "See 1 quote".to_string(),
                        n => format!("See {} quotes", compact(n)),
                    },
                    Some(Message::Nav(nav::Msg::OpenRoute(Route::Quotes(
                        uri.clone(),
                        QuoteSort::Top,
                    )))),
                ));
            }
            ui::menu(items)
        }),
        likes: count(post.like_count),
        liked,
        like: Message::Action(actions::Msg::Like(uri.clone(), toggle(liked))),
        bookmarked,
        bookmark: Message::Action(actions::Msg::Bookmark(uri.clone(), toggle(bookmarked))),
        more: Message::Popup(popups::Msg::ToggleMenu(uri.clone(), more_menu)),
        more_popup: open(more_menu).then(|| menu(app, post)),
        close: Message::Popup(popups::Msg::CloseMenu),
    })
}

/// The first post of the thread this post belongs to.
pub fn thread_root(post: &PostView) -> &AtUri {
    post.record
        .reply
        .as_ref()
        .map_or(&post.uri, |r| &r.root.uri)
}

/// The post's address on bsky.app, for opening in a browser.
pub fn web_url(post: &PostView) -> String {
    match parse_at_uri(&post.uri) {
        Some((_, _, rkey)) => format!("https://bsky.app/profile/{}/post/{rkey}", post.author.did),
        None => "https://bsky.app".into(),
    }
}

fn format_full_time(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%-I:%M %p · %b %-d, %Y")
                .to_string()
        })
        .unwrap_or_default()
}

/// The author's picture, which opens their profile; or, while they are streaming,
/// the picture in a red ring, which opens the stream.
fn author_avatar<'a>(app: &'a App, a: &'a Author) -> Element<'a, Message> {
    let now = a.status.as_ref().map(|_| chrono::Utc::now().to_rfc3339());
    let live = a
        .status
        .as_ref()
        .zip(now.as_deref())
        .and_then(|(s, now)| s.live_now(now));
    match live {
        Some(l) => ui::tip(
            ui::hand(ui::live_avatar(
                app.picture(a.avatar.as_deref()),
                pc::AVATAR,
            ))
            .on_press(Message::Nav(nav::Msg::OpenUrl(l.url.to_string()))),
            format!("Live on {}", l.service),
        ),
        None => ui::hand(avatar(app, a.avatar.as_deref(), pc::AVATAR))
            .on_press(Message::Nav(nav::Msg::OpenProfile(a.did.to_string())))
            .into(),
    }
}
