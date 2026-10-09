//! A card for a long-form article: where it comes from, its title and a way to
//! follow the publication.

use super::super::avatar::{Picture, picture};
use super::super::button::{hand, secondary};
use super::super::icon::{Icon, icon};
use super::super::surface::card;
use super::super::text::{faint, muted};
use crate::theme::{self, Tone, gap, icon_size, radius, size};
use iced::widget::{Column, Row, button, column, row, text};
use iced::{Alignment, Element, Length};

/// An article a post links to.
pub struct ArticleCard<'a, M> {
    /// The publication's name, such as a blog's.
    pub publication: &'a str,
    pub icon: Option<Picture>,
    pub title: &'a str,
    pub description: String,
    pub minutes: Option<u32>,
    pub thumb: Option<Picture>,
    pub follow: Option<Follow<M>>,
}

/// The Subscribe button: its label, its message (`None` while it is being
/// changed), and the publication's own colours as `[r, g, b]` behind and in front.
pub struct Follow<M> {
    pub subscribed: bool,
    pub press: Option<M>,
    pub colors: Option<([u8; 3], [u8; 3])>,
}

/// An article's card. Pressing it opens the article.
pub fn article_card<'a, M: Clone + 'a>(a: ArticleCard<'a, M>, open: M) -> Element<'a, M> {
    let mut source = Row::new().spacing(gap::S).align_y(Alignment::Center);
    source = match a.icon {
        Some(pic) => source.push(picture(pic, 18.0, 18.0, radius::S)),
        None => source.push(icon(Icon::Article, icon_size::S, Tone::Accent)),
    };
    source = source.push(faint(a.publication).size(size::CAPTION));
    if let Some(m) = a.minutes {
        source = source.push(faint(format!("· {m} min read")).size(size::CAPTION));
    }
    let mut info: Column<'a, M> = column![source].spacing(gap::XS);
    if !a.title.is_empty() {
        info = info.push(text(a.title).font(theme::SEMIBOLD).size(size::LARGE));
    }
    if !a.description.is_empty() {
        info = info.push(muted(a.description));
    }
    let mut actions = row![secondary("Read").on_press(open.clone())]
        .spacing(gap::S)
        .align_y(Alignment::Center);
    if let Some(f) = a.follow {
        let label = if f.subscribed {
            "Subscribed"
        } else {
            "Subscribe"
        };
        let b = if f.subscribed {
            secondary(label)
        } else {
            button(text(label).size(size::SMALL).font(theme::MEDIUM))
                .padding([gap::S - 1.0, gap::L])
                .style(theme::branded(f.colors))
        };
        actions = actions.push(b.on_press_maybe(f.press));
    }
    info = info.push(actions);
    let mut content = Row::new().spacing(gap::M).align_y(Alignment::Center);
    content = content.push(info.width(Length::Fill));
    if let Some(pic) = a.thumb {
        content = content.push(hand(picture(pic, 96.0, 96.0, radius::S)).on_press(open.clone()));
    }
    hand(card(content).padding(gap::M)).on_press(open).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Open,
        Subscribe,
    }

    fn card_with(follow: Option<Follow<Msg>>) -> Element<'static, Msg> {
        article_card(
            ArticleCard {
                publication: "Field Notes",
                icon: None,
                title: "Hello",
                description: "A first post".into(),
                minutes: Some(4),
                thumb: None,
                follow,
            },
            Msg::Open,
        )
    }

    #[test]
    fn reads_and_subscribes() {
        let follow = Follow {
            subscribed: false,
            press: Some(Msg::Subscribe),
            colors: None,
        };
        let mut ui = simulator(card_with(Some(follow)));
        assert!(ui.find("· 4 min read").is_ok());
        assert!(ui.click("Subscribe").is_ok());
        assert!(ui.click("Read").is_ok());
        let msgs: Vec<Msg> = ui.into_messages().collect();
        assert!(msgs.starts_with(&[Msg::Subscribe]));
        assert!(msgs.contains(&Msg::Open));

        let mut ui = simulator(card_with(None));
        assert!(ui.find("Subscribe").is_err());
    }
}
