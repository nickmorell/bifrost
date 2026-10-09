//! The dialog's menus: who can interact, emoji and content warnings.

use super::{Host, View};
use crate::components::{self as ui, Icon, ghost, icon, icon_button, popover};
use crate::features::composer::message::Msg;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, column, container, row, text};
use iced::{Alignment, Element, Length};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// "Everybody can interact", opening who-can-reply settings.
    pub(super) fn interaction_button(&self) -> Element<'a, M> {
        let c = self.composer;
        let lift = self.lift;
        let threads = c.reply_to.is_none();
        let i = &c.interaction;
        let limited = (threads && i.replies.is_some()) || !i.quotes;
        let label = if threads {
            i.summary()
        } else if i.quotes {
            "Quotes allowed"
        } else {
            "Quotes off"
        };
        let tint = if limited { Tone::Accent } else { Tone::Muted };
        let base = button(
            row![
                icon(
                    if limited { Icon::Lock } else { Icon::Users },
                    icon_size::SM,
                    tint
                ),
                text(label).size(size::SMALL).style(theme::tone(tint))
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        )
        .padding([gap::XS + 1.0, gap::M])
        .style(theme::ghost)
        .on_press(self.msg(Msg::ToggleInteraction));
        let popup = c.interaction_open.then(|| {
            let mut col = column![
                self.host
                    .interaction_panel(i, threads, &|i| { lift(Msg::Interaction(i)) })
            ]
            .spacing(gap::M);
            if i != self.host.default_interaction() {
                col = col.push(ghost(
                    "Use these settings for new posts",
                    self.msg(Msg::KeepInteraction(i.clone())),
                ));
            }
            container(col)
                .padding(gap::M)
                .width(340)
                .style(theme::overlay)
                .into()
        });
        popover(base, popup, self.msg(Msg::CloseMenus)).into()
    }

    pub(super) fn emoji_button(&self) -> Element<'a, M> {
        let base = ui::tip(icon_button(Icon::Mood, self.msg(Msg::ToggleEmoji)), "Emoji");
        let popup = self.composer.emoji_open.then(|| {
            let mut grid = Column::new().spacing(gap::XXS);
            for chunk in EMOJI.chunks(8) {
                let mut r = Row::new().spacing(gap::XXS);
                for e in chunk {
                    r = r.push(
                        button(text(*e).size(size::LARGE + 2.0).center())
                            .width(36)
                            .height(36)
                            .padding(0)
                            .style(theme::row_button)
                            .on_press(self.msg(Msg::Emoji(e))),
                    );
                }
                grid = grid.push(r);
            }
            container(grid).padding(gap::S).style(theme::overlay).into()
        });
        popover(base, popup, self.msg(Msg::CloseMenus)).into()
    }

    /// Content warnings apply to images, as in the official app.
    pub(super) fn warnings_button(&self) -> Element<'a, M> {
        let c = self.composer;
        if !c.has_media() {
            return Column::new().into();
        }
        let label = match c.labels.len() {
            0 => "Content warning".to_string(),
            _ => WARNINGS
                .iter()
                .filter(|(v, _)| c.labels.contains(v))
                .map(|(_, name)| *name)
                .collect::<Vec<_>>()
                .join(", "),
        };
        let tint = if c.labels.is_empty() {
            Tone::Muted
        } else {
            Tone::Warning
        };
        let base = button(
            row![
                icon(Icon::Shield, icon_size::M, tint),
                text(label).size(size::SMALL)
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        )
        .padding([gap::S - 1.0, gap::M])
        .style(theme::secondary)
        .on_press(self.msg(Msg::ToggleWarnings));
        let popup = c.warnings_open.then(|| {
            let mut col = Column::new().spacing(gap::HAIR).push(
                container(ui::faint("Adult content and graphic media"))
                    .padding([gap::XS, gap::M - 2.0]),
            );
            for (value, name) in WARNINGS {
                let on = c.labels.contains(&value);
                col = col.push(
                    button(
                        row![
                            icon(
                                if on { Icon::Check } else { Icon::Close },
                                icon_size::M,
                                if on { Tone::Accent } else { Tone::Overlay }
                            ),
                            text(name).size(size::SMALL),
                        ]
                        .spacing(gap::M - 2.0)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding([gap::S, gap::M - 2.0])
                    .style(theme::row_button)
                    .on_press(self.msg(Msg::ToggleLabel(value))),
                );
            }
            container(col)
                .padding(gap::XS)
                .width(240)
                .style(theme::overlay)
                .into()
        });
        popover(base, popup, self.msg(Msg::CloseMenus)).into()
    }
}

/// Self-labels a post's images can carry, as named in the official app.
const WARNINGS: [(&str, &str); 4] = [
    ("sexual", "Suggestive"),
    ("nudity", "Nudity"),
    ("porn", "Adult"),
    ("graphic-media", "Graphic media"),
];

/// The emoji picker's choices: the most used emoji.
const EMOJI: &[&str] = &[
    "😀",
    "😂",
    "🥹",
    "😊",
    "😍",
    "🥰",
    "😘",
    "😎",
    "🤔",
    "🫠",
    "😅",
    "😭",
    "😤",
    "😳",
    "🙃",
    "😴",
    "👍",
    "👎",
    "👏",
    "🙌",
    "🙏",
    "🤝",
    "💪",
    "👀",
    "❤️",
    "🧡",
    "💛",
    "💚",
    "💙",
    "💜",
    "🖤",
    "💔",
    "🔥",
    "✨",
    "🎉",
    "💯",
    "✅",
    "❌",
    "⚠️",
    "💡",
    "☕",
    "🍕",
    "🌱",
    "🌸",
    "🌈",
    "☀️",
    "🌙",
    "⭐",
    "🦋",
    "🐱",
    "🐶",
    "🦀",
    "🚀",
    "💻",
    "📷",
    "🎵",
    "📚",
    "✍️",
    "🎮",
    "⚽",
    "🏳️‍🌈",
    "🏳️‍⚧️",
    "🌊",
    "🌍",
];
