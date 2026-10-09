//! The cards in Settings. Everything they draw is owned or static, so each one stands
//! on its own and the screen can hand them back without borrowing the app.

use super::super::message::Msg;
use super::super::props::Prefs;
use crate::components::{self as ui, Icon, danger, ghost, icon, pill, select};
use crate::model::{LANGUAGES, Language, language};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, column, container, row, text, toggler};
use iced::{Alignment, Element, Length};

/// A preference with its explanation and a switch on the right.
pub(super) fn switch_row<'a, M: Clone + 'a>(
    title: &'static str,
    about: &'static str,
    on: bool,
    change: impl Fn(bool) -> M + 'a,
) -> Row<'a, M> {
    row![
        titled(title, about).width(Length::Fill),
        toggler(on)
            .size(size::TOGGLE)
            .style(theme::switch)
            .on_toggle(change),
    ]
    .spacing(gap::L)
    .align_y(Alignment::Center)
}

/// A preference's name with its explanation underneath.
pub(super) fn titled<'a, M: 'a>(title: &'static str, about: &'static str) -> Column<'a, M> {
    column![
        text(title).size(size::BODY).font(theme::MEDIUM),
        ui::muted(about),
    ]
    .spacing(gap::XS)
}

/// The language new posts are tagged with.
pub(super) fn posting<M: Clone + 'static>(p: &Prefs, lift: fn(Msg) -> M) -> Element<'static, M> {
    let lang = p.post_lang.clone();
    let language = row![
        titled(
            "Post language",
            "New posts are tagged with this language so people can filter by it. You can change it for a single post in the composer.",
        )
        .width(Length::Fill),
        select(LANGUAGES, language(&lang), move |l: Language| lift(
            Msg::PostLang(l.code.into())
        ))
        .placeholder(lang),
    ]
    .spacing(gap::L)
    .align_y(Alignment::Center);
    column![
        language,
        switch_row(
            "Remind me about alt text",
            "Posting an image without a description asks first. Descriptions are what people using screen readers hear.",
            p.alt_reminder,
            move |on| lift(Msg::AltReminder(on)),
        ),
    ]
    .spacing(gap::L)
    .into()
}

/// Colours and the reading font.
pub(super) fn appearance<M: Clone + 'static>(p: &Prefs, lift: fn(Msg) -> M) -> Element<'static, M> {
    let mut looks = Row::new().spacing(gap::XS);
    for (i, label) in p.appearances.iter().enumerate() {
        looks = looks.push(pill(*label, p.appearance == i, lift(Msg::Appearance(i))));
    }
    column![
        titled("Colours", "Dark, light, or whichever your system is set to."),
        looks.wrap().vertical_spacing(gap::XS),
        switch_row(
            "Proportional post text",
            "Posts and bios use your system's interface font, which reads more easily in long posts. Everything else stays monospace.",
            p.sans_reading,
            move |on| lift(Msg::SansReading(on)),
        ),
    ]
    .spacing(gap::L)
    .into()
}

/// Columns on Home.
pub(super) fn layout<M: Clone + 'static>(p: &Prefs, lift: fn(Msg) -> M) -> Element<'static, M> {
    switch_row(
        "Columns",
        "Home shows several feeds side by side, like a deck. Add, move and remove columns from Home.",
        p.deck,
        move |on| lift(Msg::Deck(on)),
    )
    .into()
}

/// Zoom and reduced motion.
pub(super) fn accessibility<M: Clone + 'static>(
    p: &Prefs,
    lift: fn(Msg) -> M,
) -> Element<'static, M> {
    let mut sizes = Row::new().spacing(gap::XS);
    for z in p.zooms {
        sizes = sizes.push(pill(
            format!("{}%", (z * 100.0).round()),
            (p.zoom - z).abs() < 0.001,
            lift(Msg::Zoom(*z)),
        ));
    }
    column![
        titled(
            "Size",
            "Makes text and everything else larger or smaller. Ctrl and + or - work anywhere.",
        ),
        sizes.wrap().vertical_spacing(gap::XS),
        switch_row(
            "Reduce motion",
            "GIFs stay still in timelines and play when you open them.",
            p.reduce_motion,
            move |on| lift(Msg::ReduceMotion(on)),
        ),
        switch_row(
            "Data saver",
            "Uses less data on slow or metered connections: pictures load only when they come into view, GIFs stay still and link cards come without pictures.",
            p.data_saver,
            move |on| lift(Msg::DataSaver(on)),
        ),
    ]
    .spacing(gap::L)
    .into()
}

/// The row that opens the Moderation screen.
pub(super) fn moderation<M: Clone + 'static>(
    muted_words: usize,
    lift: fn(Msg) -> M,
) -> Element<'static, M> {
    let words = match muted_words {
        0 => "no muted words".to_string(),
        1 => "1 muted word".to_string(),
        n => format!("{n} muted words"),
    };
    button(
        row![
            icon(Icon::Shield, icon_size::L, Tone::Muted),
            column![
                text("Moderation").size(size::BODY).font(theme::MEDIUM),
                ui::muted(format!(
                    "Content filters, {words}, muted and blocked accounts."
                )),
            ]
            .spacing(gap::XS)
            .width(Length::Fill),
            icon(Icon::ChevronRight, icon_size::M, Tone::Faint),
        ]
        .spacing(gap::M)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([gap::M, gap::M])
    .style(theme::row_button)
    .on_press(lift(Msg::Moderation))
    .into()
}

/// The integrations, each off until it is turned on.
pub(super) fn integrations<'a, M: Clone + 'static>(
    col: Column<'a, M>,
    p: &Prefs,
    lift: fn(Msg) -> M,
) -> Column<'a, M> {
    let mut col = col.push(ui::muted(
        "Optional connections to other services. Each one is off until you turn it on, and makes no network requests while off.",
    ));
    for s in &p.services {
        let id = s.id;
        col = col.push(
            ui::card(switch_row(s.name, s.description, s.on, move |on| {
                lift(Msg::Integration(id, on))
            }))
            .padding(gap::L),
        );
    }
    col
}

/// "Remove all BiFrost data" at the bottom of Settings, asking before it acts.
pub(super) fn remove_all<M: Clone + 'static>(
    confirming: bool,
    lift: fn(Msg) -> M,
) -> Element<'static, M> {
    let body: Element<'static, M> = if confirming {
        column![
            text("Remove everything?")
                .size(size::BODY)
                .font(theme::MEDIUM),
            ui::muted(
                "This signs out every account, and deletes drafts, settings and saved \
                 translation keys from this computer. Your posts on Bluesky aren't affected."
            ),
            row![
                ghost("Cancel", lift(Msg::ConfirmRemoveAll(false))),
                danger("Remove everything", lift(Msg::ConfirmRemoveAll(true))),
            ]
            .spacing(gap::S),
        ]
        .spacing(gap::S)
        .into()
    } else {
        row![
            titled(
                "Remove all BiFrost data",
                "Sign out everywhere and delete what BiFrost keeps on this computer.",
            )
            .width(Length::Fill),
            danger("Remove…", lift(Msg::RemoveAll)),
        ]
        .spacing(gap::L)
        .align_y(Alignment::Center)
        .into()
    };
    ui::card(body).padding(gap::L).into()
}

/// The keyboard shortcuts, as listed in Settings.
const SHORTCUTS: &[(&str, &str)] = &[
    ("j / k", "Next / previous post"),
    ("Enter", "Open the selected post"),
    ("l", "Like or unlike the selected post"),
    ("b", "Bookmark the selected post"),
    ("a", "Open the selected post's author"),
    ("c", "Select and copy the selected post's text"),
    ("n", "New post"),
    ("Ctrl+Enter", "Send the post you are writing"),
    ("/", "Search"),
    ("r", "Refresh"),
    ("Esc", "Close, or go back"),
    ("Alt+Left", "Go back (the mouse back button works too)"),
    ("Tab", "Next text box (Shift+Tab: previous)"),
    (
        "Ctrl + / -",
        "Make everything larger or smaller (Ctrl+0: normal size)",
    ),
];

pub(super) fn shortcuts<M: Clone + 'static>() -> Element<'static, M> {
    let mut keys = Column::new().spacing(gap::XS + 2.0);
    for (key, what) in SHORTCUTS {
        keys = keys.push(
            row![
                container(
                    container(text(*key).size(size::SMALL).font(theme::MEDIUM))
                        .padding([2.0, gap::S])
                        .style(theme::keycap)
                )
                .width(110),
                ui::muted(*what),
            ]
            .spacing(gap::M)
            .align_y(Alignment::Center),
        );
    }
    keys.into()
}
