//! The moderation screen and the report dialog.

mod report;

use super::{Moderation, Msg};
use crate::components::{self as ui, Icon, divider, icon, icon_button, pill};
use crate::model::{LabelVisibility, PeopleKind, PrefEdit, Preferences};
use crate::moderation;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, column, row, space, text, toggler};
use iced::{Alignment, Element, Length};

/// The moderation screen, unscrolled.
pub struct Screen<'a> {
    pub moderation: &'a Moderation,
    /// Your preferences, which hold the filters and muted words.
    pub prefs: &'a Preferences,
}

impl<'a> Screen<'a> {
    pub fn view(self) -> Element<'a, Msg> {
        let prefs = self.prefs;
        let mut col = Column::new().spacing(gap::M).padding([gap::S, gap::XL]);

        let mut filters = Column::new().spacing(gap::M).push(
            row![
                column![
                    text("Allow adult content").size(size::BODY).font(theme::MEDIUM),
                    ui::muted("Lets you choose below whether to see adult content. When off, it is always hidden."),
                ]
                .spacing(gap::XS)
                .width(Length::Fill),
                toggler(prefs.adult_content_enabled)
                    .size(size::TOGGLE)
                    .style(theme::switch)
                    .on_toggle(|on| Msg::Edit(PrefEdit::AdultContent(on))),
            ]
            .spacing(gap::L)
            .align_y(Alignment::Center),
        );
        for (label, name, about) in moderation::FILTERS {
            let choice: Element<'a, Msg> =
                if moderation::is_adult(label) && !prefs.adult_content_enabled {
                    ui::faint("Hidden").size(size::SMALL).into()
                } else {
                    let current = moderation::label_setting(prefs, label);
                    let mut pills = Row::new().spacing(gap::XS);
                    for (vis, word) in [
                        (LabelVisibility::Hide, "Hide"),
                        (LabelVisibility::Warn, "Warn"),
                        (LabelVisibility::Ignore, "Show"),
                    ] {
                        pills = pills.push(pill(
                            word,
                            current == vis,
                            Msg::Edit(PrefEdit::Label(label.to_string(), vis)),
                        ));
                    }
                    pills.into()
                };
            filters = filters.push(divider()).push(
                row![
                    column![
                        text(name).size(size::BODY).font(theme::MEDIUM),
                        ui::muted(about),
                    ]
                    .spacing(gap::XS)
                    .width(Length::Fill),
                    choice,
                ]
                .spacing(gap::L)
                .align_y(Alignment::Center),
            );
        }
        col = col
            .push(ui::eyebrow("Content filters"))
            .push(ui::card(filters).padding(gap::L));

        let can_add = !self
            .moderation
            .muted_word
            .trim()
            .trim_start_matches('#')
            .is_empty();
        let mut words = Column::new().spacing(gap::S).push(
            row![
                ui::field("Word, phrase or #tag", &self.moderation.muted_word)
                    .on_input(Msg::WordInput)
                    .on_submit(Msg::AddWord),
                ui::secondary("Mute").on_press_maybe(can_add.then_some(Msg::AddWord)),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        );
        if prefs.muted_words.is_empty() {
            words = words.push(ui::muted(
                "Posts containing a muted word or tag are hidden from your feeds and notifications.",
            ));
        }
        for w in &prefs.muted_words {
            words = words.push(
                row![
                    text(w.value.clone()).size(size::BODY).width(Length::Fill),
                    icon_button(
                        Icon::Close,
                        Msg::Edit(PrefEdit::UnmuteWord(w.value.clone()))
                    ),
                ]
                .spacing(gap::S)
                .align_y(Alignment::Center),
            );
        }
        col = col
            .push(space::vertical().height(gap::L))
            .push(ui::eyebrow("Muted words and tags"))
            .push(ui::card(words).padding(gap::L));

        let link = |i: Icon, label: &'static str, kind: PeopleKind| {
            button(
                row![
                    icon(i, icon_size::L, Tone::Muted),
                    text(label).size(size::BODY).width(Length::Fill),
                    icon(Icon::ChevronRight, icon_size::M, Tone::Faint),
                ]
                .spacing(gap::M)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([gap::S + 2.0, gap::M])
            .style(theme::row_button)
            .on_press(Msg::People(kind))
        };
        col = col
            .push(space::vertical().height(gap::L))
            .push(ui::eyebrow("Accounts"))
            .push(
                ui::card(
                    column![
                        link(Icon::VolumeOff, "Muted accounts", PeopleKind::Muted),
                        link(Icon::Ban, "Blocked accounts", PeopleKind::Blocked),
                    ]
                    .spacing(gap::XXS),
                )
                .padding(gap::XS),
            )
            .push(ui::muted(
                "These settings are saved to your Bluesky account, so the official app follows them too.",
            ));

        col.into()
    }
}
