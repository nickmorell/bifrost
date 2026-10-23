//! The frame of a post in a list or thread: avatar, thread lines, header, the parts
//! the caller fills in, and the action bar (`actions.rs`). Notices for hidden posts
//! and media are in `notice.rs`.

mod actions;
mod notice;

pub use actions::{Actions, actions};
pub use notice::{hidden, hidden_media, hidden_quote};

use super::feedback::tip;
use super::icon::{Icon, icon};
use super::popover::context_menu;
use super::text::faint;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, button, container, row, text};
use iced::{Alignment, Element, Length, Padding};

/// The size of a post's profile picture.
pub const AVATAR: f32 = 40.0;
/// How far each level of replies is indented.
pub const GUIDE: f32 = gap::XL + AVATAR / 2.0 + 1.0;

/// A post to frame.
pub struct Card<'a, M> {
    /// The author's picture, already clickable.
    pub avatar: Element<'a, M>,
    /// Header, text, embeds and actions, top to bottom.
    pub body: Column<'a, M>,
    /// A line above the post, such as "Reposted by …".
    pub above: Option<Element<'a, M>>,
    /// Continue a thread line from the post above into this post's avatar.
    pub line_above: bool,
    /// Draw a thread line from this post's avatar down to the next post.
    pub line_below: bool,
    /// What clicking empty space in the post sends. `None` for the post a thread is
    /// about, which is already open.
    pub open: Option<M>,
    /// The menu right-clicking the post opens.
    pub menu: Option<ContextMenu<'a, M>>,
}

/// A post's right-click menu.
pub struct ContextMenu<'a, M> {
    /// What right-clicking sends.
    pub open: M,
    /// The menu, while it is open.
    pub popup: Option<Element<'a, M>>,
    /// What clicking outside the open menu sends.
    pub close: M,
}

/// A post in its frame.
pub fn card<'a, M: Clone + 'a>(c: Card<'a, M>) -> Element<'a, M> {
    let mut side = Column::new()
        .width(AVATAR)
        .align_x(Alignment::Center)
        .spacing(gap::XS);
    let mut body = container(c.body.width(Length::Fill));
    if c.line_above {
        side = side.push(thread_line(Length::Fixed(gap::L - gap::XS)));
        body = body.padding(Padding::ZERO.top(gap::L));
    }
    side = side.push(c.avatar);
    if c.line_below {
        side = side.push(thread_line(Length::Fill));
    }
    let content = row![side, body].spacing(gap::M + 2.0);

    let mut outer = Column::new().spacing(gap::S);
    if let Some(above) = c.above {
        outer = outer.push(indented(above));
    }
    outer = outer.push(content);

    // Posts joined by a thread line meet without padding, so the line is unbroken.
    let pad = |joined: bool| if joined { 0.0 } else { gap::L };
    let frame = container(outer)
        .padding(
            Padding::from([0.0, gap::XL])
                .top(pad(c.line_above))
                .bottom(pad(c.line_below)),
        )
        .width(Length::Fill);
    // Clicking empty space in a post opens it; buttons and links inside take
    // precedence.
    let post: Element<'a, M> = match c.open {
        None => frame.into(),
        Some(open) => button(frame)
            .padding(0)
            .style(theme::post)
            .on_press(open)
            .into(),
    };
    match c.menu {
        None => post,
        Some(m) => context_menu(post, m.popup, m.open, m.close).into(),
    }
}

/// A stand-in for a post in a thread that cannot be shown, joined to its
/// neighbours by the thread line.
pub fn placeholder<'a, M: 'a>(
    label: &'a str,
    line_above: bool,
    line_below: bool,
) -> Element<'a, M> {
    let mut side = Column::new().width(AVATAR).align_x(Alignment::Center);
    if line_above {
        side = side.push(thread_line(Length::Fixed(gap::M)));
    }
    side = side.push(
        container(icon(Icon::EyeOff, icon_size::S, Tone::Faint))
            .center(26)
            .style(theme::placeholder(13.0)),
    );
    if line_below {
        side = side.push(thread_line(Length::Fill));
    }
    let pad = |joined: bool| if joined { 0.0 } else { gap::M };
    container(
        row![
            side,
            container(faint(label).size(size::SMALL)).padding(Padding::ZERO.top(if line_above {
                gap::M + 4.0
            } else {
                4.0
            })),
        ]
        .spacing(gap::M + 2.0),
    )
    .padding(
        Padding::from([0.0, gap::XL])
            .top(pad(line_above))
            .bottom(pad(line_below)),
    )
    .width(Length::Fill)
    .into()
}

/// The line running down the left of a reply's own replies, under its avatar.
pub fn thread_guide<'a, M: 'a>() -> Element<'a, M> {
    container(thread_line(Length::Fill))
        .padding(Padding::ZERO.left(gap::XL + AVATAR / 2.0 - 1.0))
        .width(GUIDE)
        .height(Length::Fill)
        .into()
}

/// The vertical line joining a reply to the post above it.
fn thread_line<'a, M: 'a>(height: Length) -> Element<'a, M> {
    container(iced::widget::space().width(2).height(height))
        .style(theme::thread_line)
        .into()
}

/// Lines a repost or pin notice up with the post text, past the avatar.
fn indented<'a, M: 'a>(e: Element<'a, M>) -> Element<'a, M> {
    container(e)
        .padding(Padding::ZERO.left(AVATAR + gap::M + 2.0))
        .into()
}

/// A small icon and line of quiet text, e.g. "Reposted by …".
pub fn context_line<'a, M: Clone + 'a>(i: Icon, label: String, msg: M) -> Element<'a, M> {
    button(
        row![icon(i, icon_size::XS, Tone::Faint), faint(label)]
            .spacing(gap::XS + 2.0)
            .align_y(Alignment::Center),
    )
    .padding(0)
    .style(theme::bare)
    .on_press(msg)
    .into()
}

/// "Pinned", over a post pinned to its author's profile.
pub fn pinned<'a, M: 'a>() -> Element<'a, M> {
    row![icon(Icon::Pin, icon_size::XS, Tone::Faint), faint("Pinned")]
        .spacing(gap::XS + 2.0)
        .align_y(Alignment::Center)
        .into()
}

/// Who wrote a post and when.
pub struct Header<'a> {
    pub name: &'a str,
    pub handle: &'a str,
    /// How long ago, and the exact time for its tooltip. Left out for the post a
    /// thread is about, which shows the full time below instead.
    pub time: Option<(String, String)>,
    /// The post a thread is about, with a larger name.
    pub large: bool,
    /// A verification check after the name: `Some(true)` for a trusted verifier.
    pub verified: Option<bool>,
    /// The width of the post.
    pub width: f32,
}

/// Name, handle and time on one line. Long names and handles are cut short with an
/// ellipsis to fit the post's width; the font is monospace, so counting characters
/// measures them.
pub fn header<'a, M: Clone + 'a>(h: Header<'a>, on_name: M) -> Element<'a, M> {
    let name_size = if h.large { size::LARGE } else { size::BODY };
    let handle = format!("@{}", h.handle);
    // What is left beside the avatar, less the gaps between the parts.
    let mut room = h.width - 2.0 * gap::XL - AVATAR - gap::M - 2.0 - 3.0 * gap::S;
    if let Some((t, _)) = &h.time {
        room -= (t.chars().count() + 1) as f32 * char_width(size::SMALL) + gap::S;
    }
    if h.verified.is_some() {
        room -= icon_size::S + gap::S;
    }
    let name_max = (room * 0.6 / char_width(name_size)) as usize;
    let name = shorten(h.name, name_max.max(8));
    room -= name.chars().count() as f32 * char_width(name_size);
    let handle_max = (room / char_width(size::SMALL)).max(0.0) as usize;
    let name = button(text(name).font(theme::SEMIBOLD).size(name_size))
        .padding(0)
        .style(theme::bare)
        .on_press(on_name);
    let mut r = row![name].spacing(gap::S).align_y(Alignment::Center);
    if let Some(verifier) = h.verified {
        r = r.push(super::verified(verifier, icon_size::S));
    }
    // Too little room for a useful part of the handle: the name stands alone.
    if handle_max >= 8 {
        r = r.push(faint(shorten(&handle, handle_max)));
    }
    if let Some((t, full)) = h.time {
        r = r.push(faint("·")).push(tip(faint(t), full));
    }
    r.wrap().into()
}

/// The advance of one character of the monospace font at `size`.
fn char_width(size: f32) -> f32 {
    size * 0.6
}

/// `s` cut to at most `max` characters, ending in an ellipsis if it was cut.
fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::column;
    use iced_test::simulator;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Open,
        Menu,
        Profile,
    }

    fn post(open: Option<Msg>) -> Element<'static, Msg> {
        card(Card {
            avatar: faint("pic").into(),
            body: column![header(
                Header {
                    name: "Ada",
                    handle: "ada.test",
                    time: Some(("5m".into(), "Oct 4".into())),
                    large: false,
                    verified: Some(false),
                    width: 680.0,
                },
                Msg::Profile,
            )],
            above: Some(pinned()),
            line_above: false,
            line_below: true,
            open,
            menu: Some(ContextMenu {
                open: Msg::Menu,
                popup: None,
                close: Msg::Menu,
            }),
        })
    }

    #[test]
    fn clicks_open_the_post_or_the_author() {
        let mut ui = simulator(post(Some(Msg::Open)));
        assert!(ui.find("Pinned").is_ok() && ui.find("@ada.test").is_ok());
        assert!(ui.click("Ada").is_ok());
        assert!(ui.click("5m").is_ok());
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            [Msg::Profile, Msg::Open]
        );
        // The open post does nothing when clicked, but right-clicking it opens its menu.
        let mut ui = simulator(post(None));
        assert!(ui.click("5m").is_ok());
        assert!(ui.into_messages().next().is_none());
        let mut ui = simulator(post(None));
        ui.point_at(iced::Point::new(300.0, 20.0));
        let _ = ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
            iced::mouse::Button::Right,
        ))]);
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Msg::Menu]);
    }

    #[test]
    fn long_names_are_cut_to_fit() {
        assert_eq!(shorten("abcdef", 4), "abc…");
        assert_eq!(shorten("abc", 4), "abc");
        let narrow = header(
            Header {
                name: "A very long display name indeed",
                handle: "a-very-long-handle.bsky.social",
                time: None,
                large: true,
                verified: None,
                width: 300.0,
            },
            (),
        );
        let mut ui = simulator(narrow);
        // Too narrow for the handle: the name stands alone.
        assert!(ui.find("@a-very-long-handle.bsky.social").is_err());
    }

    #[test]
    fn placeholders_and_guides_draw() {
        let mut ui = simulator(placeholder::<()>("Deleted post", true, true));
        assert!(ui.find("Deleted post").is_ok());
        assert!(
            simulator(thread_guide::<()>())
                .snapshot(&iced::Theme::Dark)
                .is_ok()
        );
    }
}
