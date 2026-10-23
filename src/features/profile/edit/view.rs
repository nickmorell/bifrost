//! How the "Edit profile" dialog looks.

use super::{Editor, MAX_BIO, MAX_NAME, Msg, Pic, Picture};
use crate::api::richtext::grapheme_len;
use crate::components::{self as ui, Icon, ghost, icon_button};
use crate::features::Pictures;
use crate::theme::{self, gap, radius, size};
use iced::widget::{column, container, image, row, space, stack, text, text_editor};
use iced::{Alignment, Element, Length};

const DIALOG_WIDTH: f32 = 560.0;
const BANNER_HEIGHT: f32 = 150.0;
const AVATAR: f32 = 84.0;
/// Taller than this, the form scrolls between its title and its buttons.
const DIALOG_MAX_HEIGHT: f32 = 760.0;

impl Editor {
    /// The picture shown for `pic`: the new one picked, or the one on the profile now.
    fn preview(&self, pictures: &impl Pictures, pic: Pic) -> Option<image::Handle> {
        match self.picture(pic) {
            Picture::New(_, preview) => Some(preview.clone()),
            Picture::Current(Some(url)) => match pictures.picture(Some(url)) {
                Some(ui::Picture::Loaded(h)) => Some(h),
                _ => None,
            },
            _ => None,
        }
    }

    /// The dialog; `lift` turns its messages into the app's.
    pub fn view<'a, M: Clone + 'a>(
        &'a self,
        pictures: &impl Pictures,
        lift: fn(Msg) -> M,
    ) -> Element<'a, M> {
        let wrap = lift;
        let busy = self.saving;

        let banner: Element<'a, M> = match self.preview(pictures, Pic::Banner) {
            Some(h) => image(h)
                .width(Length::Fill)
                .height(BANNER_HEIGHT)
                .content_fit(iced::ContentFit::Cover)
                .border_radius(radius::L)
                .into(),
            None => container(space::vertical().height(BANNER_HEIGHT))
                .width(Length::Fill)
                .style(theme::placeholder(radius::L))
                .into(),
        };
        let avatar: Element<'a, M> = match self.preview(pictures, Pic::Avatar) {
            Some(h) => image(h)
                .width(AVATAR)
                .height(AVATAR)
                .content_fit(iced::ContentFit::Cover)
                .border_radius(AVATAR / 2.0)
                .into(),
            None => ui::avatar(None, AVATAR),
        };
        let pic_buttons = |pic: Pic, present: bool| {
            let mut r = row![ghost(
                match pic {
                    Pic::Avatar => "Change picture",
                    Pic::Banner => "Change banner",
                },
                wrap(Msg::Pick(pic))
            )]
            .spacing(gap::XS);
            if present {
                r = r.push(ghost("Remove", wrap(Msg::Remove(pic))));
            }
            r
        };

        let name_len = grapheme_len(&self.name);
        let bio = self.bio();
        let bio_len = grapheme_len(&bio);
        let counter = |n: usize, max: usize| {
            let t = text(format!("{n}/{max}")).size(size::CAPTION);
            if n > max {
                t.style(theme::danger_text)
            } else {
                t.style(theme::faint)
            }
        };

        let title = row![
            text("Edit profile").size(size::TITLE).font(theme::SEMIBOLD),
            space::horizontal(),
            icon_button(Icon::Close, wrap(Msg::Cancel)),
        ]
        .align_y(Alignment::Center);
        let mut col = column![
            stack![
                banner,
                container(
                    container(pic_buttons(Pic::Banner, self.banner.present()))
                        .style(theme::media_badge)
                )
                .padding(gap::S)
                .align_right(Length::Fill)
                .align_bottom(BANNER_HEIGHT),
            ],
            row![avatar, pic_buttons(Pic::Avatar, self.avatar.present())]
                .spacing(gap::L)
                .align_y(Alignment::Center),
            column![
                row![
                    ui::faint("Display name"),
                    space::horizontal(),
                    counter(name_len, MAX_NAME)
                ],
                ui::field("Your name", &self.name).on_input(move |v| wrap(Msg::Name(v))),
            ]
            .spacing(gap::XS),
            column![
                row![
                    ui::faint("Bio"),
                    space::horizontal(),
                    counter(bio_len, MAX_BIO)
                ],
                text_editor(&self.bio)
                    .on_action(move |a| wrap(Msg::Bio(a)))
                    .placeholder("Tell people about yourself")
                    .font(theme::FONT)
                    .size(size::BODY)
                    .padding(gap::M)
                    .height(120)
                    .style(theme::editor),
            ]
            .spacing(gap::XS),
            self.links.view(wrap),
        ]
        .spacing(gap::M)
        .padding(iced::Padding::ZERO.right(gap::M));

        if let Some(e) = &self.error {
            col = col.push(text(e).size(size::SMALL).style(theme::danger_text));
        }
        let buttons = row![
            space::horizontal(),
            ghost("Cancel", wrap(Msg::Cancel)),
            ui::primary(if busy { "Saving…" } else { "Save" })
                .on_press_maybe((!busy && self.valid()).then_some(wrap(Msg::Save))),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center);

        let footer = container(buttons).padding(iced::Padding::ZERO.top(gap::M));
        let body = ui::footed(ui::scroll(col).height(Length::Shrink), footer);
        container(column![title, body].spacing(gap::M))
            .padding(gap::XL)
            .width(DIALOG_WIDTH)
            .max_height(DIALOG_MAX_HEIGHT)
            .style(theme::dialog)
            .into()
    }
}
