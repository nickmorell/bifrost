//! The composer's views. The dialog shows the post being replied to or quoted and
//! who-can-reply settings, which the app draws, so these views build the app's own
//! message type `M` through [`View::lift`].

mod menus;
mod parts;
mod schedule;
mod thread;

use super::message::Msg;
use super::state::{Composer, MAX_IMAGES};
use crate::api::richtext::{self, MAX_GRAPHEMES};
use crate::components::{self as ui, Icon, ghost, icon_button};
use crate::features::Pictures;
use crate::model::{Interaction, PostRecord};
use crate::model::{LANGUAGES, Language, language};
use crate::theme::{self, gap, size};
use iced::widget::{Column, column, container, row, text, text_editor};
use iced::{Alignment, Element, Length, Padding};

const DIALOG_WIDTH: f32 = 640.0;
const DIALOG_MAX_HEIGHT: f32 = 760.0;

/// What the composer's views need from the app around them.
pub trait Host<M>: Pictures {
    /// A post's text, drawn as in timelines.
    fn rich_text<'a>(&'a self, record: &'a PostRecord) -> Element<'a, M>;
    /// Your own avatar.
    fn my_avatar(&self) -> Option<&str>;
    /// Hashtags starting with `prefix` that you used or saw lately.
    fn tag_suggestions(&self, prefix: &str) -> Vec<String>;
    /// Who can reply and quote new posts, as set in Settings.
    fn default_interaction(&self) -> &Interaction;
    /// The who-can-reply choices; `threads` when the post starts a thread.
    fn interaction_panel<'a>(
        &'a self,
        current: &Interaction,
        threads: bool,
        on_change: &dyn Fn(Interaction) -> M,
    ) -> Element<'a, M>;
}

/// The composer as a dialog; the app floats it over whatever screen is open.
pub struct View<'a, H, M> {
    pub composer: &'a Composer,
    pub host: &'a H,
    /// Turns the composer's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    fn msg(&self, m: Msg) -> M {
        (self.lift)(m)
    }

    fn avatar(&self, url: Option<&str>, size: f32) -> Element<'a, M> {
        ui::avatar(self.host.picture(url), size)
    }

    pub fn dialog(&self) -> Element<'a, M> {
        let c = self.composer;
        let lift = self.lift;
        let text_now = c.text();
        let confirming = c.confirm_close;
        let len = richtext::grapheme_len(text_now.trim_end());
        let over = len > MAX_GRAPHEMES;

        let title = if c.reply_to.is_some() {
            "Reply"
        } else if c.quote.is_some() {
            "Quote post"
        } else {
            "New post"
        };
        let bar = row![
            icon_button(Icon::Close, self.msg(Msg::Cancel)),
            text(title).size(size::TITLE).font(theme::SEMIBOLD),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center)
        .padding(Padding::from([gap::M, gap::L]).left(gap::M))
        .height(56);

        let mut col = Column::new()
            .spacing(gap::L)
            .padding(Padding::from([gap::S, gap::XL]));

        if let Some(p) = &c.reply_to {
            col = col.push(self.context_card(p, "Replying to"));
        }

        col = col.push(
            row![
                self.avatar(self.host.my_avatar(), 40.0),
                text_editor(&c.content)
                    .placeholder(if c.reply_to.is_some() {
                        "Write your reply"
                    } else {
                        "What's on your mind?"
                    })
                    .on_action(move |a| lift(Msg::Edit(a)))
                    .key_binding(move |press| {
                        use iced::keyboard::{Key, key::Named};
                        use iced::widget::text_editor::Binding;
                        match press.key.as_ref() {
                            // Ctrl+Enter (Cmd+Enter on a Mac) posts.
                            Key::Named(Named::Enter) if press.modifiers.command() => {
                                Some(Binding::Custom(lift(Msg::Submit)))
                            }
                            Key::Character(c) if c == "v" && press.modifiers.command() => {
                                Some(Binding::Custom(lift(Msg::Paste)))
                            }
                            Key::Named(Named::Escape) => {
                                Some(Binding::Custom(lift(if confirming {
                                    Msg::KeepEditing
                                } else {
                                    Msg::Cancel
                                })))
                            }
                            _ => Binding::from_key_press(press),
                        }
                    })
                    .size(size::LARGE)
                    .line_height(1.55)
                    .height(180)
                    .padding(gap::M + 2.0)
                    .style(theme::editor),
            ]
            .spacing(gap::M + 2.0),
        );

        if let Some(t) = self.thread_panel(over) {
            col = col.push(t);
        }
        if let Some(s) = self.suggestions() {
            col = col.push(s);
        }
        if let Some(t) = self.tags() {
            col = col.push(t);
        }
        if let Some(a) = self.attachments() {
            col = col.push(a);
        }

        if let Some(q) = &c.quote {
            col = col.push(self.context_card(q, "Quoting"));
        }

        col = col.push(self.interaction_button());

        let counter = if c.threaded() {
            text(format!("{} posts", c.parts().len()))
        } else {
            text(format!("{len}/{MAX_GRAPHEMES}"))
        }
        .size(size::SMALL)
        .style(if over && !c.threaded() {
            theme::danger_text
        } else {
            theme::faint
        });
        let can_post = !c.posting && c.can_post(text_now.trim_end());
        // Questions, errors and the Post button stay in view below the scrolling part,
        // however long the post or thread is.
        let mut footer = Column::new()
            .spacing(gap::S)
            .padding(Padding::from([gap::M, gap::XL]).top(gap::S));
        if confirming {
            footer = footer.push(
                ui::card(
                    row![
                        ui::muted("Keep this post as a draft?").width(Length::Fill),
                        ghost("Keep editing", self.msg(Msg::KeepEditing)),
                        ui::danger("Discard", self.msg(Msg::Discard)),
                        ui::primary("Save draft").on_press(self.msg(Msg::SaveDraft)),
                    ]
                    .spacing(gap::S)
                    .align_y(Alignment::Center),
                )
                .padding([gap::S, gap::M]),
            );
        }

        if let Some(a) = self.alt_check() {
            footer = footer.push(a);
        }
        if let Some(e) = &c.error {
            footer = footer.push(text(e).size(size::SMALL).style(theme::danger_text));
        } else if c.posting && c.video.is_some() {
            footer = footer.push(ui::faint(
                "Uploading the video. Bluesky gets it ready for playing before the post \
                 goes up, which can take a minute.",
            ));
        }

        footer = footer.push(
            row![
                ui::tip(
                    icon_button(Icon::Photo, self.msg(Msg::AddImages)).on_press_maybe(
                        (c.images.len() < MAX_IMAGES && c.video.is_none())
                            .then(|| self.msg(Msg::AddImages))
                    ),
                    "Add images. You can also paste or drop them here.",
                ),
                ui::tip(
                    icon_button(Icon::Video, self.msg(Msg::AddVideo))
                        .on_press_maybe((!c.has_media()).then(|| self.msg(Msg::AddVideo))),
                    "Add a video (up to 3 minutes). You can also drop it here.",
                ),
                self.emoji_button(),
                self.warnings_button(),
                ui::select(LANGUAGES, language(&c.lang), move |l: Language| {
                    lift(Msg::Lang(l.code.into()))
                })
                .placeholder(c.lang.clone())
                .width(120),
                iced::widget::space::horizontal(),
                self.schedule_button(can_post),
                counter,
                ui::primary(if c.posting { "Posting…" } else { "Post" })
                    .on_press_maybe(can_post.then(|| self.msg(Msg::Submit))),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        );

        container(column![
            bar,
            ui::footed(ui::scroll(col).height(Length::Shrink), footer)
        ])
        .width(DIALOG_WIDTH)
        .max_height(DIALOG_MAX_HEIGHT)
        .style(theme::dialog)
        .into()
    }
}
