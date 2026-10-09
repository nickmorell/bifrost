//! The chat feature's views. A chat message can carry a post, which the app draws,
//! so these views build the app's own message type `M` through [`View::lift`]
//! instead of returning [`Msg`] elements to be mapped.

mod dialogs;
mod list;
mod thread;

use super::message::Msg;
use super::state::Chats;
use crate::components::{self as ui, Icon, ghost};
use crate::features::Pictures;
use crate::model::{AllowIncoming, Author, Did, PostRecord, QuotedPost};
use crate::theme::{self, gap, radius, size};
use iced::widget::{Row, column, container, row, text};
use iced::{Alignment, Element, Length, Padding};

/// What the chat views need from the app around them.
pub trait Host<M>: Pictures {
    /// A message's text, drawn as post text is.
    fn rich_text<'a>(&'a self, body: &'a PostRecord) -> Element<'a, M>;
    /// A post sent in a chat, drawn as a quote.
    fn quoted_post<'a>(&'a self, post: &'a QuotedPost) -> Element<'a, M>;
    /// Whether the account on screen signed in through the browser.
    fn browser_sign_in(&self) -> bool;
}

/// The chat views, drawing [`Chats`] for the signed-in account.
pub struct View<'a, H, M> {
    pub chats: &'a Chats,
    pub host: &'a H,
    pub me: &'a Did,
    /// Turns the feature's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    fn msg(&self, m: Msg) -> M {
        (self.lift)(m)
    }

    fn avatar(&self, url: Option<&str>, size: f32) -> Element<'a, M> {
        ui::avatar(self.host.picture(url), size)
    }

    /// Something that failed to load, with "Sign in again" or "Try again".
    fn retry(&self, why: impl Into<String>) -> Element<'a, M> {
        ui::retry(why, self.msg(Msg::SignInAgain), self.msg(Msg::Retry))
    }

    /// The buttons in a chat's title bar.
    pub fn bar_actions(&self, id: &'a str) -> Element<'a, M> {
        let Some(c) = self.chats.convos.get(id) else {
            return Row::new().into();
        };
        let mut r = Row::new().spacing(gap::XS).align_y(Alignment::Center);
        if !c.is_group(self.me)
            && let Some(other) = c.others(self.me).next()
        {
            r = r.push(ui::icon_action(
                Icon::User,
                "View profile",
                self.msg(Msg::OpenProfile(other.did.clone())),
            ));
        }
        r = r.push(if c.muted {
            ui::icon_action(
                Icon::BellOff,
                "Unmute chat",
                self.msg(Msg::Mute(id.into(), false)),
            )
        } else {
            ui::icon_action(
                Icon::Bell,
                "Mute chat",
                self.msg(Msg::Mute(id.into(), true)),
            )
        });
        r.push(if self.chats.confirm_leave.as_deref() == Some(id) {
            ui::danger("Really leave?", self.msg(Msg::Leave(id.into()))).into()
        } else {
            ui::danger_icon_action(Icon::Leave, "Leave chat", self.msg(Msg::Leave(id.into())))
        })
        .into()
    }

    /// The "Who can message you" setting.
    pub fn settings(&self) -> Element<'a, M> {
        let lift = self.lift;
        row![
            column![
                text("Who can message you")
                    .size(size::BODY)
                    .font(theme::MEDIUM),
                ui::muted("People you already chat with can always reply."),
            ]
            .spacing(gap::XS)
            .width(Length::Fill),
            ui::select(&AllowIncoming::ALL, self.chats.allow, move |a| lift(
                Msg::SetAllow(a)
            ))
            .placeholder("Loading…")
            .width(190),
        ]
        .spacing(gap::L)
        .align_y(Alignment::Center)
        .into()
    }

    fn no_access(&self) -> Element<'a, M> {
        let (why, button) = if self.host.browser_sign_in() {
            (
                "You signed in with your browser before BiFrost asked to read your chats. Sign out and sign in again, and allow chats when Bluesky asks.",
                ui::secondary("Sign out").on_press(self.msg(Msg::SignOut)),
            )
        } else {
            (
                "Bluesky only lets an app password open chats when \"Allow access to your direct messages\" was ticked when it was made. Make a new app password with that ticked, then sign in with it.",
                ui::secondary("App password settings").on_press(self.msg(Msg::OpenUrl(
                    "https://bsky.app/settings/app-passwords".into(),
                ))),
            )
        };
        ui::card(
            column![
                text("BiFrost can't open your chats")
                    .size(size::LARGE)
                    .font(theme::SEMIBOLD),
                ui::muted(why),
                row![button, ghost("Try again", self.msg(Msg::Refresh))].spacing(gap::S),
            ]
            .spacing(gap::M),
        )
        .padding(gap::XL)
        .into()
    }

    /// The chat list's scrolling, which loads more chats near its end.
    fn scroll(
        &self,
        key: &str,
        content: impl Into<Element<'a, M>>,
        at_end: Option<M>,
    ) -> Element<'a, M> {
        let lift = self.lift;
        ui::scroll(container(ui::reading(content)).padding(Padding::ZERO.bottom(gap::XXL)))
            .id(key.to_string())
            .on_scroll(move |v| match &at_end {
                Some(m) if v.relative_offset().y > 0.9 => m.clone(),
                _ => lift(Msg::ListScrolled(v)),
            })
            .into()
    }

    /// Two avatars overlapping, for a group.
    fn stack_avatars(&self, a: Option<&'a Author>, b: Option<&'a Author>) -> Element<'a, M> {
        let pic = |x: Option<&'a Author>| self.avatar(x.and_then(|a| a.avatar.as_deref()), 28.0);
        iced::widget::stack![
            container(pic(a)),
            container(container(pic(b)).style(|t: &iced::Theme| {
                container::Style {
                    border: iced::Border::default()
                        .rounded(radius::PILL)
                        .width(2)
                        .color(theme::palette(t).surface),
                    ..Default::default()
                }
            }))
            .padding(Padding::ZERO.left(14.0).top(14.0)),
        ]
        .width(42)
        .height(42)
        .into()
    }
}
