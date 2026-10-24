//! Cards for things a post links to: a web page, an integration such as a YouTube
//! video, or a feed, list or starter pack.

mod article;

pub use article::{ArticleCard, Follow, article_card};

use super::avatar::{Picture, picture};
use super::button::{hand, secondary};
use super::icon::{Icon, icon};
use super::surface::{card, outlined};
use super::text::{faint, muted};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, radius, size};
use iced::widget::{Column, Row, column, row, text};
use iced::{Alignment, Element, Length};

/// A link to a web page.
pub struct Link<'a> {
    /// The site, such as "example.com".
    pub domain: String,
    pub title: &'a str,
    pub description: String,
    /// The page's preview picture, if it has one.
    pub thumb: Option<Picture>,
}

/// A web page's card: its picture, site, title and description.
pub fn link_card<'a, M: Clone + 'a>(link: Link<'a>, open: M) -> Element<'a, M> {
    let mut info = column![
        row![
            icon(Icon::External, icon_size::XXS, Tone::Faint),
            faint(link.domain).size(size::CAPTION),
        ]
        .spacing(gap::XS + 2.0)
        .align_y(Alignment::Center)
    ]
    .spacing(gap::XS);
    if !link.title.is_empty() {
        info = info.push(text(link.title).font(theme::MEDIUM).size(size::BODY));
    }
    if !link.description.is_empty() {
        info = info.push(muted(link.description));
    }
    let mut content = Row::new().spacing(gap::M).align_y(Alignment::Center);
    if let Some(pic) = link.thumb {
        content = content.push(picture(pic, 88.0, 88.0, radius::S));
    }
    content = content.push(info.width(Length::Fill));
    hand(card(content).padding(gap::S + 2.0))
        .on_press(open)
        .into()
}

/// A link an integration knows how to show.
pub struct Integration {
    pub provider: &'static str,
    pub title: Option<String>,
    /// A line under the title, such as a repository's description.
    pub text: Option<String>,
    pub subtitle: Option<String>,
    /// Short facts after the subtitle, such as "1.2K stars".
    pub facts: Vec<String>,
    /// Streaming now: a LIVE mark beside the provider.
    pub live: bool,
    /// What the open button says, such as "Watch on YouTube".
    pub action_label: String,
    pub thumb: Option<Picture>,
    pub art: Art,
}

/// Where an integration card puts its picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Art {
    /// Wide across the top, shown once it has loaded: videos and photos.
    Banner,
    /// A square beside the text: album art.
    Cover,
}

/// An integration's card: a large picture, title and an open button.
pub fn integration_card<'a, M: Clone + 'a>(card_: Integration, open: M) -> Element<'a, M> {
    let mut top = Row::new()
        .push(faint(card_.provider).size(size::CAPTION))
        .spacing(gap::S)
        .align_y(Alignment::Center);
    if card_.live {
        top = top.push(super::video_player::live_mark());
    }
    let mut info = column![top].spacing(gap::XS);
    if let Some(t) = card_.title {
        info = info.push(text(t).font(theme::MEDIUM).size(size::BODY));
    }
    if let Some(t) = card_.text {
        info = info.push(muted(t));
    }
    let line: Vec<String> = card_.subtitle.into_iter().chain(card_.facts).collect();
    if !line.is_empty() {
        info = info.push(faint(line.join(" · ")));
    }
    let button = secondary(card_.action_label).on_press(open.clone());
    let mut line = Row::new().spacing(gap::M).align_y(Alignment::Center);
    let mut content = Column::new().spacing(gap::M);
    match (card_.art, card_.thumb) {
        (Art::Banner, Some(Picture::Loaded(handle))) => {
            content = content.push(
                hand(
                    iced::widget::image(handle)
                        .width(Length::Fill)
                        .height(280)
                        .content_fit(iced::ContentFit::Cover)
                        .border_radius(radius::S),
                )
                .on_press(open.clone()),
            );
        }
        (Art::Cover, Some(pic)) => {
            line = line.push(hand(picture(pic, 88.0, 88.0, radius::S)).on_press(open.clone()));
        }
        _ => {}
    }
    content = content.push(line.push(info.width(Length::Fill)).push(button));
    card(content).padding(gap::S + 2.0).into()
}

/// A one-line card for a feed, list or starter pack.
pub fn record_link<'a, M: Clone + 'a>(i: Icon, label: String, open: M) -> Element<'a, M> {
    hand(outlined(
        row![icon(i, icon_size::M, Tone::Accent), muted(label)]
            .spacing(gap::S)
            .align_y(Alignment::Center),
    ))
    .on_press(open)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Open,
    }

    fn clicked(view: Element<'_, Msg>, label: &str) -> Vec<Msg> {
        let mut ui = simulator(view);
        assert!(ui.click(label).is_ok(), "no {label:?} to click");
        ui.into_messages().collect()
    }

    #[test]
    fn cards_open_their_link() {
        let link = Link {
            domain: "example.com".into(),
            title: "A page",
            description: "About things".into(),
            thumb: Some(Picture::Waiting { pulse: false }),
        };
        assert_eq!(clicked(link_card(link, Msg::Open), "A page"), [Msg::Open]);

        let yt = Integration {
            provider: "YouTube",
            title: Some("A video".into()),
            text: None,
            subtitle: None,
            facts: Vec::new(),
            live: true,
            action_label: "Watch".into(),
            thumb: None,
            art: Art::Banner,
        };
        assert_eq!(
            clicked(integration_card(yt, Msg::Open), "Watch"),
            [Msg::Open]
        );

        let song = Integration {
            provider: "Spotify",
            title: Some("A song".into()),
            text: None,
            subtitle: Some("Song".into()),
            facts: vec!["2 plays".into()],
            live: false,
            action_label: "Listen".into(),
            thumb: Some(Picture::Waiting { pulse: false }),
            art: Art::Cover,
        };
        assert_eq!(
            clicked(integration_card(song, Msg::Open), "Listen"),
            [Msg::Open]
        );

        let list = record_link(Icon::List, "List · Friends by @a.test".into(), Msg::Open);
        assert_eq!(clicked(list, "List · Friends by @a.test"), [Msg::Open]);
    }
}
