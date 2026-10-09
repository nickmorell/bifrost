//! Parts of the dialog: suggestions, attachments, link card and the posts replied to
//! or quoted.

use super::{Host, View};
use crate::api::DraftVideo;
use crate::components::{self as ui, Icon, Picture, ghost, icon, icon_button};
use crate::features::composer::message::Msg;
use crate::model::PostView;
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, radius, size};
use iced::widget::{Column, Row, button, column, container, image, row, text};
use iced::{Alignment, Element, Length};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// People matching the `@partial` being typed.
    pub(super) fn suggestions(&self) -> Option<Element<'a, M>> {
        let found = &self.composer.suggestions;
        if found.is_empty() {
            return None;
        }
        let mut list = Column::new().spacing(gap::XXS);
        for a in found {
            list = list.push(
                button(
                    row![
                        self.avatar(a.avatar.as_deref(), 24.0),
                        text(a.name()).size(size::BODY),
                        ui::faint(format!("@{}", a.handle)),
                    ]
                    .spacing(gap::S)
                    .align_y(Alignment::Center),
                )
                .style(theme::row_button)
                .padding([gap::S, gap::M])
                .width(Length::Fill)
                .on_press(self.msg(Msg::PickMention(a.handle.clone()))),
            );
        }
        Some(
            container(list)
                .padding(gap::XS)
                .style(theme::overlay)
                .width(Length::Fill)
                .into(),
        )
    }

    /// Hashtags matching the `#partial` being typed.
    pub(super) fn tags(&self) -> Option<Element<'a, M>> {
        let partial = self.composer.tag_partial.as_deref()?;
        let tags = self.host.tag_suggestions(partial);
        if tags.is_empty() {
            return None;
        }
        let mut list = Row::new().spacing(gap::XS);
        for t in tags {
            list = list.push(
                button(text(format!("#{t}")).size(size::SMALL))
                    .style(theme::pill(false))
                    .padding([gap::XS + 1.0, gap::M])
                    .on_press(self.msg(Msg::PickTag(t))),
            );
        }
        Some(list.wrap().into())
    }

    /// The images with their alt text, the video, or the link card.
    pub(super) fn attachments(&self) -> Option<Element<'a, M>> {
        let c = self.composer;
        if !c.images.is_empty() {
            let lift = self.lift;
            let parts = if c.threaded() { c.parts().len() } else { 0 };
            let mut imgs = Row::new().spacing(gap::M);
            for (i, a) in c.images.iter().enumerate() {
                let on_post = (parts > 1).then(|| {
                    ui::tip(
                        ghost(
                            format!("On post {}", a.part.min(parts - 1) + 1),
                            self.msg(Msg::ImagePart(i)),
                        ),
                        "Move it to the next post of the thread",
                    )
                });
                imgs = imgs.push(
                    column![
                        image(a.preview.clone())
                            .width(150)
                            .height(150)
                            .content_fit(iced::ContentFit::Cover)
                            .border_radius(radius::M),
                        ui::field("Alt text", &a.alt)
                            .on_input(move |v| lift(Msg::Alt(i, v)))
                            .size(size::SMALL)
                            .width(150),
                        ghost("Remove", self.msg(Msg::RemoveImage(i))),
                    ]
                    .push(on_post)
                    .spacing(gap::S),
                );
            }
            return Some(imgs.wrap().into());
        }
        if let Some(v) = &c.video {
            return Some(self.video_card(v));
        }
        let url = c.card_url.as_ref()?;
        let card: Element<'a, M> = match &c.card {
            Some(card) => {
                let mut info = column![text(&card.title).size(size::BODY).font(theme::MEDIUM)]
                    .spacing(gap::XS);
                if !card.description.is_empty() {
                    info = info.push(ui::muted(&card.description));
                }
                let mut r = Row::new().spacing(gap::M).align_y(Alignment::Center);
                if let Some(thumb) = card.image_url.as_deref() {
                    let pic = self
                        .host
                        .picture(Some(thumb))
                        .unwrap_or(Picture::Waiting { pulse: false });
                    r = r.push(ui::picture(pic, 80.0, 80.0, radius::S));
                }
                r.push(info.width(Length::Fill))
                    .push(icon_button(Icon::Close, self.msg(Msg::RemoveCard)))
                    .into()
            }
            None => row![
                ui::faint(format!("Loading preview for {url}…")).width(Length::Fill),
                ghost("Skip", self.msg(Msg::RemoveCard)),
            ]
            .align_y(Alignment::Center)
            .into(),
        };
        Some(ui::card(card).padding(gap::S + 2.0).into())
    }

    /// The post being replied to or quoted.
    pub(super) fn context_card(&self, p: &'a PostView, label: &'static str) -> Element<'a, M> {
        ui::outlined(
            column![
                row![
                    ui::faint(label),
                    self.avatar(p.author.avatar.as_deref(), 18.0),
                    text(p.author.name()).size(size::SMALL).font(theme::MEDIUM),
                    ui::faint(format!("@{}", p.author.handle)),
                ]
                .spacing(gap::S)
                .align_y(Alignment::Center),
                self.host.rich_text(&p.record),
            ]
            .spacing(gap::S),
        )
        .into()
    }

    /// The picked video, with what it shows and how big it is.
    fn video_card(&self, v: &'a DraftVideo) -> Element<'a, M> {
        let lift = self.lift;
        let mut facts = Vec::new();
        if let Some(d) = v.info.duration {
            let s = d.round() as u64;
            facts.push(format!("{}:{:02}", s / 60, s % 60));
        }
        if let Some((w, h)) = v.info.size {
            facts.push(format!("{w}×{h}"));
        }
        facts.push(format!("{:.1} MB", v.bytes.len() as f64 / 1_048_576.0));
        ui::card(
            column![
                row![
                    icon(Icon::Video, icon_size::XL, Tone::Accent),
                    column![
                        text(&v.name).size(size::BODY).font(theme::MEDIUM),
                        ui::faint(facts.join(" · ")),
                    ]
                    .spacing(gap::XXS)
                    .width(Length::Fill),
                    ghost("Remove", self.msg(Msg::RemoveVideo)),
                ]
                .spacing(gap::M)
                .align_y(Alignment::Center),
                ui::field(
                    "Alt text: describe the video for people who can't see it",
                    &v.alt
                )
                .on_input(move |a| lift(Msg::VideoAlt(a)))
                .size(size::SMALL),
            ]
            .spacing(gap::S + 2.0),
        )
        .padding(gap::M)
        .into()
    }
}
