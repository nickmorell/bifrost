//! Links to long-form articles, drawn as article cards that open in the reader.

use crate::app::util::picture_of;
use crate::app::{App, Message, articles};
use crate::components as ui;
use crate::model::{ArticleRef, ExternalView};
use iced::Element;

/// The article a link points at, when BiFrost can open it in the reader.
pub(in crate::app) fn article_of(ext: &ExternalView) -> Option<ArticleRef> {
    ext.document()
        .map(|d| ArticleRef::Standard {
            uri: d.clone(),
            link: Some(ext.uri.clone()),
        })
        .or_else(|| ArticleRef::whitewind(&ext.uri))
}

/// An article card, for a link Bluesky knows is an article or a WhiteWind entry.
pub(super) fn article_view<'a>(
    app: &'a App,
    ext: &'a ExternalView,
) -> Option<Element<'a, Message>> {
    let article = article_of(ext);
    if ext.source.is_none() && article.is_none() {
        return None;
    }
    let open = match article {
        Some(a) => articles::read(a),
        None => Message::Nav(crate::app::nav::Msg::OpenUrl(ext.uri.clone())),
    };
    let source = ext.source.as_ref();
    let follow = ext.publication().map(|p| {
        let (subscribed, busy) = app.articles.subscription(p);
        let colors = source
            .and_then(|s| s.theme.as_ref())
            .and_then(|t| t.accent.zip(t.accent_foreground))
            .map(|(bg, fg)| ([bg.r, bg.g, bg.b], [fg.r, fg.g, fg.b]));
        ui::Follow {
            subscribed,
            press: (!busy).then(|| Message::Articles(articles::Msg::Toggle(p.clone()))),
            colors,
        }
    });
    let publication = source.map_or(
        if ext.uri.contains("whtwnd.com") {
            "WhiteWind"
        } else {
            "Article"
        },
        |s| s.title.as_str(),
    );
    Some(ui::article_card(
        ui::ArticleCard {
            publication,
            icon: source
                .and_then(|s| s.icon.as_deref())
                .map(|u| picture_of(app, u)),
            title: &ext.title,
            description: ui::truncate(&ext.description, 200),
            minutes: ext.reading_time,
            thumb: ext
                .thumb
                .as_deref()
                .filter(|_| !app.settings.data_saver)
                .map(|t| picture_of(app, t)),
            follow,
        },
        open,
    ))
}
