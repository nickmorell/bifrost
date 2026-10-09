//! The article reader and the Articles screen.

use super::{Articles, Msg, Reading};
use crate::components::{self as ui, Icon, Picture, faint, muted, status};
use crate::features::Pictures;
use crate::model::{Article, ArticleRef, ArticleSummary, Block};
use crate::theme::{self, Look, Tone, gap, icon_size, radius, size};
use iced::widget::{Column, column, container, image, row, text};
use iced::{Alignment, ContentFit, Element, Length};

/// The day something was published, such as "Oct 8, 2026".
fn day(iso: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%b %-d, %Y")
                .to_string()
        })
        .unwrap_or_default()
}

/// The article in the reader, unscrolled.
pub struct Reader<'a, P> {
    pub articles: &'a Articles,
    pub pictures: &'a P,
    pub look: Look,
}

impl<'a, P: Pictures> Reader<'a, P> {
    pub fn view(self) -> Element<'a, Msg> {
        match &self.articles.reader {
            None | Some((_, Reading::Loading)) => status("Loading the article…"),
            Some((article, Reading::Failed(why))) => {
                let why = status(format!("Couldn't open the article. {why}"));
                match article.link() {
                    Some(link) => column![
                        why,
                        ui::secondary("Open in browser").on_press(Msg::OpenUrl(link.to_string())),
                    ]
                    .align_x(Alignment::Center)
                    .width(Length::Fill)
                    .into(),
                    None => why,
                }
            }
            Some((_, Reading::Ready(a))) => self.article(a),
        }
    }

    fn article(&self, a: &'a Article) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(gap::M).padding([gap::S, gap::XL]);
        if let Some(p) = &a.publication {
            let mut source = row![].spacing(gap::S).align_y(Alignment::Center);
            source = match self.pictures.picture(p.icon.as_deref()) {
                Some(pic) => source.push(ui::picture(pic, 22.0, 22.0, radius::S)),
                None => source.push(ui::icon(Icon::Article, icon_size::M, Tone::Accent)),
            };
            source = source.push(muted(p.name.as_str()).width(Length::Fill));
            let (on, busy) = self.articles.subscription(&p.uri);
            let label = if on { "Subscribed" } else { "Subscribe" };
            let press = (!busy).then(|| Msg::Toggle(p.uri.clone()));
            let button = if on {
                ui::secondary(label)
            } else {
                ui::primary(label)
            };
            col = col.push(source.push(button.on_press_maybe(press)));
        }
        col = col.push(
            text(a.title.as_str())
                .font(theme::BOLD)
                .size(size::HEADING)
                .line_height(1.3),
        );
        if !a.description.is_empty() {
            col = col.push(muted(a.description.as_str()).size(size::LARGE));
        }
        let mut meta = vec![day(&a.published)];
        meta.push(format!("{} min read", a.reading_minutes()));
        meta.retain(|m| !m.is_empty());
        col = col.push(faint(meta.join(" · ")));
        if let Some(Picture::Loaded(handle)) = self.pictures.picture(a.cover.as_deref()) {
            col = col.push(
                image(handle)
                    .width(Length::Fill)
                    .height(260)
                    .content_fit(ContentFit::Cover)
                    .border_radius(radius::M),
            );
        }
        let (font, body) = self.look.reading(size::LARGE);
        for b in &a.blocks {
            col = col.push(match b {
                Block::Heading(t) => Element::from(
                    text(t.as_str())
                        .font(theme::SEMIBOLD)
                        .size(size::TITLE)
                        .line_height(1.4),
                ),
                Block::Paragraph(t) => text(t.as_str())
                    .font(font)
                    .size(body)
                    .line_height(1.7)
                    .into(),
                Block::Item(t) => row![
                    faint("•").size(body),
                    text(t.as_str()).font(font).size(body).line_height(1.7)
                ]
                .spacing(gap::S)
                .into(),
                Block::Code(t) => ui::outlined(text(t.as_str()).size(size::BODY))
                    .width(Length::Fill)
                    .into(),
            });
        }
        if let Some(url) = &a.url {
            col = col.push(
                container(ui::secondary("Open on the web").on_press(Msg::OpenUrl(url.clone())))
                    .padding([gap::L, 0.0]),
            );
        }
        col.into()
    }
}

/// The Articles screen: the newest articles from publications you subscribe to.
pub struct List<'a> {
    pub articles: &'a Articles,
}

impl<'a> List<'a> {
    pub fn view(self) -> Element<'a, Msg> {
        let a = self.articles;
        let head = row![
            muted("New from the publications you subscribe to.").width(Length::Fill),
            ui::secondary("Refresh").on_press_maybe((!a.refreshing).then_some(Msg::Refresh)),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center);
        let mut col = Column::new()
            .spacing(gap::S)
            .padding([0.0, gap::L])
            .push(head);
        if a.subscribed.is_empty() {
            return col
                .push(status(
                    "No subscriptions yet. Press Subscribe on an article to see its new posts here.",
                ))
                .into();
        }
        match &a.latest {
            None => col.push(status("Loading articles…")).into(),
            Some(list) if list.is_empty() => col
                .push(status("Nothing new from your subscriptions."))
                .into(),
            Some(list) => {
                for s in list {
                    col = col.push(summary(s));
                }
                col.into()
            }
        }
    }
}

fn summary(s: &ArticleSummary) -> Element<'_, Msg> {
    let mut info = column![
        faint(format!("{} · {}", s.publication, day(&s.published))).size(size::CAPTION),
        text(s.title.as_str())
            .font(theme::SEMIBOLD)
            .size(size::BODY),
    ]
    .spacing(gap::XS);
    if !s.description.is_empty() {
        info = info.push(muted(ui::truncate(&s.description, 200)));
    }
    ui::hand(ui::card(info).padding([gap::M, gap::L]).width(Length::Fill))
        .on_press(Msg::Read(ArticleRef::Standard {
            uri: s.uri.clone(),
            link: None,
        }))
        .into()
}
