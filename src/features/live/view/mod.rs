//! The Live screens: who is live now, and a streamer's page with their stream and
//! chat. The stream plays in the app's video player, so these views build the app's
//! own message type `M` through [`View::lift`] and ask the [`Host`] to draw it.

mod board;
mod channel;
mod chat;

use super::message::Msg;
use super::state::Live;
use crate::components::{self as ui, Picture};
use crate::features::Pictures;
use crate::model::PostRecord;
use crate::theme::{self, gap, size};
use iced::widget::{container, text};
use iced::{Element, Length};

/// What the Live views need from the app around them.
pub trait Host<M>: Pictures {
    /// A chat message's text, with its links and mentions, drawn as post text is.
    fn rich_text<'a>(&'a self, body: &'a PostRecord) -> Element<'a, M>;
    /// Whether the video player is playing something under `key`.
    fn playing(&self, key: &str) -> bool;
    /// The stream the video player plays under `key`, `w` by `h`; or, while it plays
    /// something else, the stream's still with a play button that sends `watch`.
    fn player<'a>(
        &'a self,
        key: &str,
        thumb: Option<&'a str>,
        watch: M,
        w: f32,
        h: f32,
    ) -> Element<'a, M>;
}

/// The Live views, drawing [`Live`].
pub struct View<'a, H, M> {
    pub live: &'a Live,
    pub host: &'a H,
    /// The time now, as RFC 3339, for telling who is live.
    pub now: String,
    /// Turns the feature's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// Another handle on the same views, for closures that outlive `self`.
    fn copy(&self) -> Self {
        Self {
            live: self.live,
            host: self.host,
            now: self.now.clone(),
            lift: self.lift,
        }
    }

    fn msg(&self, m: Msg) -> M {
        (self.lift)(m)
    }

    fn avatar(&self, url: Option<&str>, size: f32) -> Element<'a, M> {
        ui::avatar(self.host.picture(url), size)
    }

    /// A picture `w` by `h`, or a still placeholder when there is none.
    fn still(&self, url: Option<&str>, w: f32, h: f32) -> Element<'a, M> {
        let pic = self
            .host
            .picture(url)
            .unwrap_or(Picture::Waiting { pulse: false });
        ui::picture(pic, w, h, theme::radius::M)
    }

    /// Something that failed to load, with "Sign in again" or "Try again".
    fn retry(&self, why: impl Into<String>, again: Msg) -> Element<'a, M> {
        ui::retry(why, self.msg(Msg::SignInAgain), self.msg(again))
    }
}

/// "123 watching" on a dark badge, for over a stream's picture.
fn viewers<'a, M: 'a>(n: u64) -> Element<'a, M> {
    container(
        text(format!("{} watching", ui::compact(n)))
            .size(size::CAPTION)
            .style(theme::on_media),
    )
    .padding([1.0, gap::S])
    .style(theme::media_badge)
    .into()
}

/// A heading over a part of a page.
fn heading<'a, M: 'a>(label: &'a str) -> Element<'a, M> {
    container(text(label).size(size::LARGE).font(theme::SEMIBOLD))
        .width(Length::Fill)
        .into()
}
