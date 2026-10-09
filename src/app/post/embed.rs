//! What a post embeds: links, integration cards, quoted posts, feeds and lists.

use super::linked::post_text;
use super::web_url;
use crate::app::util::{avatar, full_time, picture_of, relative_time};
use crate::app::{App, Message, nav, popups};
use crate::components::{self as ui, Icon, Picture, media, post_card as pc};
use crate::integrations::{IntegrationCard, Shape, Shown};
use crate::model::{Embed, EmbeddedRecord, ExternalView, FeedSource, PostView, QuotedPost};
use crate::moderation::{Context, Decision};
use crate::theme::{self, gap, size};
use iced::widget::{Column, column, row, text};
use iced::{Alignment, Element};

pub(super) fn embed_view<'a>(
    app: &'a App,
    embed: &'a Embed,
    post: &'a PostView,
    decision: &Decision,
    revealed: bool,
) -> Element<'a, Message> {
    let blur = decision.blur_media.as_ref().filter(|_| !revealed);
    match embed {
        Embed::Images { images } => match blur {
            Some(reason) => pc::hidden_media(
                reason,
                Message::Popup(popups::Msg::Reveal(post.uri.clone())),
            ),
            None => crate::app::media::images_view(app, images),
        },
        Embed::Video {
            playlist,
            thumbnail,
            alt,
            aspect_ratio,
        } => match blur {
            Some(reason) => pc::hidden_media(
                reason,
                Message::Popup(popups::Msg::Reveal(post.uri.clone())),
            ),
            None => crate::app::video::video_view(
                app,
                &post.uri,
                playlist,
                thumbnail.as_deref(),
                alt.as_deref(),
                aspect_ratio
                    .as_ref()
                    .map(|r| r.width as f32 / r.height.max(1) as f32),
                web_url(post),
            ),
        },
        Embed::External { external } => external_view(app, external),
        Embed::Record { record } => record_view(app, record, 0),
        Embed::RecordWithMedia { record, media } => column![
            embed_view(app, media, post, decision, revealed),
            record_view(app, &record.record, 0),
        ]
        .spacing(gap::S)
        .into(),
        Embed::Other => Column::new().into(),
    }
}

fn domain_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_string())
        })
        .unwrap_or_default()
}

fn external_view<'a>(app: &'a App, ext: &'a ExternalView) -> Element<'a, Message> {
    let alt = if ext.description.is_empty() {
        &ext.title
    } else {
        &ext.description
    };
    if crate::app::media::is_gif_url(&ext.uri) {
        return crate::app::media::gif_view(app, &ext.uri, alt);
    }
    match app.integration_embed(&ext.uri, Some(&ext.title)) {
        Some(Shown::Card(card)) => return integration_view(app, card, ext.thumb.as_deref()),
        Some(Shown::Gif(url)) => return crate::app::media::gif_view(app, &url, alt),
        None => {}
    }
    if let Some(card) = super::article::article_view(app, ext) {
        return card;
    }
    let link = ui::Link {
        domain: domain_of(&ext.uri),
        title: &ext.title,
        description: ui::truncate(&ext.description, 160),
        thumb: ext
            .thumb
            .as_deref()
            .filter(|_| !app.settings.data_saver)
            .map(|t| picture_of(app, t)),
    };
    ui::link_card(link, Message::Nav(nav::Msg::OpenUrl(ext.uri.clone())))
}

/// An integration's card. Without a picture from the service it shows the link's own.
fn integration_view<'a>(
    app: &'a App,
    card: IntegrationCard,
    link_thumb: Option<&str>,
) -> Element<'a, Message> {
    let thumb = card
        .thumbnail_url
        .as_deref()
        .or(link_thumb)
        .map(|u| picture_of(app, u));
    let art = match card.shape {
        Shape::Banner => ui::Art::Banner,
        Shape::Cover => ui::Art::Cover,
    };
    let player = app.music_view(&card.open_url);
    let open = Message::Nav(nav::Msg::OpenUrl(card.open_url.clone()));
    let card = ui::integration_card(
        ui::Integration {
            provider: card.provider,
            title: card.title,
            subtitle: card.subtitle,
            action_label: card.action_label,
            thumb,
            art,
        },
        open,
    );
    match player {
        Some(p) => column![card, p].spacing(gap::S).into(),
        None => card,
    }
}

/// A quoted post, feed or list. A quote inside a quote is shown one level deep.
fn record_view<'a>(app: &'a App, record: &'a EmbeddedRecord, depth: u8) -> Element<'a, Message> {
    let notice = |t: String| -> Element<'a, Message> { ui::outlined(ui::muted(t)).into() };
    match record {
        EmbeddedRecord::Post(q) => quoted_post(app, q, depth),
        EmbeddedRecord::NotFound => notice("Quoted post was deleted.".into()),
        EmbeddedRecord::Blocked => notice("Quoted post is from a blocked account.".into()),
        EmbeddedRecord::Detached => notice("Quoted post was removed by its author.".into()),
        EmbeddedRecord::FeedGenerator {
            uri,
            display_name,
            creator,
        } => ui::record_link(
            Icon::Hash,
            format!("Feed · {display_name} by @{}", creator.handle),
            Message::Nav(nav::Msg::OpenFeed(
                FeedSource::Generator(uri.clone()),
                display_name.clone(),
            )),
        ),
        EmbeddedRecord::List { uri, name, creator } => ui::record_link(
            Icon::List,
            format!("List · {name} by @{}", creator.handle),
            Message::Lists(crate::app::lists::Msg::OpenList(uri.clone())),
        ),
        EmbeddedRecord::StarterPack {
            uri,
            record,
            creator,
        } => ui::record_link(
            Icon::Users,
            format!("Starter pack · {} by @{}", record.name, creator.handle),
            Message::Lists(crate::app::lists::Msg::OpenPack(uri.clone())),
        ),
        EmbeddedRecord::Other => Column::new().into(),
    }
}

pub(in crate::app) fn quoted_post<'a>(
    app: &'a App,
    q: &'a QuotedPost,
    depth: u8,
) -> Element<'a, Message> {
    let revealed = app.revealed.contains(&q.uri);
    // Quotes follow the same moderation settings as posts.
    let decision = crate::moderation::decide(
        &app.prefs,
        &q.author,
        &q.value,
        &q.labels,
        &app.my_did,
        Context::Feed,
    );
    if let Some(reason) = decision.hide.as_ref().filter(|_| !revealed) {
        return pc::hidden_quote(reason, Message::Popup(popups::Msg::Reveal(q.uri.clone())));
    }

    let mut col = column![
        row![
            avatar(app, q.author.avatar.as_deref(), 18.0),
            text(q.author.name()).font(theme::MEDIUM).size(size::SMALL),
            ui::faint(format!("@{}", q.author.handle)),
            ui::faint("·"),
            ui::tip(
                ui::faint(relative_time(&q.indexed_at)),
                full_time(&q.indexed_at)
            ),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center)
        .wrap()
    ]
    .spacing(gap::S);
    if !q.value.text.is_empty() {
        col = col.push(if depth == 0 {
            post_text(app.look(), &q.value, size::BODY - 0.5)
        } else {
            ui::muted(ui::truncate(&q.value.text, 200)).into()
        });
    }

    let blur = decision.blur_media.as_ref().filter(|_| !revealed);
    for e in &q.embeds {
        col = quoted_embed(app, col, e, q, blur, depth);
    }

    ui::hand(ui::outlined(col))
        .on_press(Message::Nav(nav::Msg::OpenThread(q.uri.clone())))
        .into()
}

/// One embed inside a quoted post, shown smaller than in a post of its own.
fn quoted_embed<'a>(
    app: &'a App,
    col: Column<'a, Message>,
    e: &'a Embed,
    q: &'a QuotedPost,
    blur: Option<&String>,
    depth: u8,
) -> Column<'a, Message> {
    match e {
        Embed::Images { images } => match blur {
            Some(reason) => col.push(pc::hidden_media(
                reason,
                Message::Popup(popups::Msg::Reveal(q.uri.clone())),
            )),
            None => col.push(crate::app::media::thumbs_view(app, images)),
        },
        Embed::Video { thumbnail, .. } => match blur {
            Some(reason) => col.push(pc::hidden_media(
                reason,
                Message::Popup(popups::Msg::Reveal(q.uri.clone())),
            )),
            None => col.push(media::video_thumb(
                thumbnail
                    .as_deref()
                    .map_or(Picture::Waiting { pulse: false }, |t| picture_of(app, t)),
            )),
        },
        Embed::External { external } => col.push(external_view(app, external)),
        Embed::Record { record } if depth == 0 => col.push(record_view(app, record, 1)),
        Embed::Record { .. } => col.push(ui::faint("Quotes another post")),
        Embed::RecordWithMedia { record, media } => {
            let col = quoted_embed(app, col, media, q, blur, depth);
            if depth == 0 {
                col.push(record_view(app, &record.record, 1))
            } else {
                col.push(ui::faint("Quotes another post"))
            }
        }
        Embed::Other => col,
    }
}
