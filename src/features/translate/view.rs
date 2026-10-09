//! The translation under a post, and the Translation card in Settings.

use super::{Ctx, Msg, State, Translations, is_foreign, translate};
use crate::components::{self as ui, Icon, ghost, icon, pill};
use crate::model::{LANGUAGES, Language, PostView, TranslateService as Service, language};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, column, container, row, space, text, text_input};
use iced::{Alignment, Element, Length};

/// Under a post's text: its translation, or a quiet "Translate" link when it is in
/// another language. Nothing otherwise.
pub fn under_post<'a>(
    t: &'a Translations,
    post: &'a PostView,
    text_size: f32,
    ctx: &Ctx,
) -> Option<Element<'a, Msg>> {
    let hide = || {
        button(ui::faint("Hide").size(size::CAPTION))
            .padding(0)
            .style(theme::bare)
            .on_press(Msg::Hide(post.uri.clone()))
    };
    match t.posts.get(&post.uri) {
        None if is_foreign(post, &ctx.to) => Some(
            button(
                row![
                    icon(Icon::Language, icon_size::XS, Tone::Faint),
                    ui::faint("Translate").size(size::CAPTION),
                ]
                .spacing(gap::XS)
                .align_y(Alignment::Center),
            )
            .padding(0)
            .style(theme::bare)
            .on_press(translate(post))
            .into(),
        ),
        None => None,
        Some(State::Loading) => Some(ui::faint("Translating…").size(size::CAPTION).into()),
        Some(State::Failed(e)) => Some(
            column![
                text(e.clone())
                    .size(size::CAPTION)
                    .style(theme::warning_text),
                row![
                    button(ui::faint("Try again").size(size::CAPTION))
                        .padding(0)
                        .style(theme::bare)
                        .on_press(translate(post)),
                    hide(),
                ]
                .spacing(gap::M),
            ]
            .spacing(gap::XS)
            .into(),
        ),
        Some(State::Done(done)) => {
            let from = done.from.as_deref().map_or_else(
                || "Translated by ".into(),
                |c| match language(c) {
                    Some(l) => format!("Translated from {} by ", l.name),
                    None => format!("Translated from {c} by "),
                },
            );
            Some(
                container(
                    column![
                        text(done.text.clone()).size(text_size),
                        row![
                            icon(Icon::Language, icon_size::XXS, Tone::Faint),
                            ui::faint(format!("{from}{}", ctx.service.name())).size(size::CAPTION),
                            space::horizontal(),
                            hide(),
                        ]
                        .spacing(gap::XS)
                        .align_y(Alignment::Center),
                    ]
                    .spacing(gap::S),
                )
                .padding([gap::S, gap::M])
                .width(Length::Fill)
                .style(theme::raised)
                .into(),
            )
        }
    }
}

/// The Translation card in Settings.
pub fn settings<'a>(t: &'a Translations, ctx: &Ctx) -> Element<'a, Msg> {
    let (service, to) = (ctx.service, ctx.to.clone());
    let mut services = Row::new().spacing(gap::XS);
    for s in Service::ALL {
        services = services.push(pill(s.name(), s == service, Msg::Service(s)));
    }
    let about = match service {
        Service::MyMemory => {
            "Free, with no account. Posts are sent to MyMemory to translate. It needs the post's language tag, which most posts have, and allows a few thousand words a day."
        }
        Service::DeepL => {
            "Good quality. Needs your own API key; a free DeepL API account gives 500,000 characters a month."
        }
        Service::LibreTranslate => {
            "Open source, and can run on your own computer or server. Some public servers need an API key."
        }
        Service::Browser => {
            "Opens the post in Google Translate in your browser. Nothing is sent from BiFrost."
        }
    };
    let mut col = Column::new()
        .spacing(gap::L)
        .push(
            row![
                column![
                    text("Translate into").size(size::BODY).font(theme::MEDIUM),
                    ui::muted("Posts in other languages get a Translate link under them."),
                ]
                .spacing(gap::XS)
                .width(Length::Fill),
                ui::select(LANGUAGES, language(&to), |l: Language| {
                    Msg::To(l.code.into())
                })
                .placeholder(to.clone()),
            ]
            .spacing(gap::L)
            .align_y(Alignment::Center),
        )
        .push(
            column![
                text("Service").size(size::BODY).font(theme::MEDIUM),
                services.wrap().vertical_spacing(gap::XS),
                ui::muted(about),
            ]
            .spacing(gap::S),
        );
    if service == Service::LibreTranslate {
        col = col.push(
            column![
                ui::faint("Server address"),
                text_input("https://libretranslate.example", &ctx.server)
                    .on_input(Msg::Server)
                    .padding(gap::S + 2.0)
                    .size(size::BODY)
                    .style(theme::input),
            ]
            .spacing(gap::XS),
        );
        if let Some(why) = crate::api::insecure_server(&ctx.server) {
            col = col.push(
                text(why.replace("password", "API key"))
                    .size(size::CAPTION)
                    .style(theme::warning_text),
            );
        }
    }
    if matches!(service, Service::DeepL | Service::LibreTranslate) {
        let saved = t.has_key.get(&service).copied().unwrap_or(false);
        let label = match (service.needs_key(), saved) {
            (_, true) => {
                "API key (saved; type a new one to replace it, or save empty to remove it)"
            }
            (true, false) => "API key",
            (false, false) => "API key, if the server needs one",
        };
        col = col.push(
            column![
                ui::faint(label),
                row![
                    text_input(
                        if saved {
                            "••••••••"
                        } else {
                            "Paste your key"
                        },
                        &t.key_input
                    )
                    .on_input(Msg::KeyInput)
                    .on_submit(Msg::SaveKey)
                    .secure(true)
                    .padding(gap::S + 2.0)
                    .size(size::BODY)
                    .style(theme::input),
                    ghost("Save", Msg::SaveKey),
                ]
                .spacing(gap::S)
                .align_y(Alignment::Center),
                ui::faint("Kept in your system's secure storage, like your login."),
            ]
            .spacing(gap::XS),
        );
    }
    col.into()
}
