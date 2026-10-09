//! Long text: the offer to split it into a thread, and the thread's preview. Also
//! the question asked before posting images without alt text.

use super::{Host, View};
use crate::components::{self as ui, ghost};
use crate::features::composer::message::Msg;
use crate::features::composer::split::BREAK;
use crate::theme::{gap, size};
use iced::widget::{Column, column, row, text};
use iced::{Alignment, Element, Length};

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// "Post without alt text?", while it is being asked.
    pub(super) fn alt_check(&self) -> Option<Element<'a, M>> {
        let c = self.composer;
        if !c.confirm_alt {
            return None;
        }
        let what = match (c.missing_alt(), c.video.is_some()) {
            (_, true) => "The video has no description.".to_string(),
            (1, _) => "An image has no description.".to_string(),
            (n, _) => format!("{n} images have no description."),
        };
        Some(
            ui::card(
                column![
                    text(what).size(size::BODY),
                    ui::muted(
                        "People using screen readers hear the description instead of seeing the picture."
                    ),
                    row![
                        iced::widget::space::horizontal(),
                        ghost("Post anyway", self.msg(Msg::PostAnyway)),
                        ui::primary("Add descriptions").on_press(self.msg(Msg::AddAlt)),
                    ]
                    .spacing(gap::S)
                    .align_y(Alignment::Center),
                ]
                .spacing(gap::S),
            )
            .padding([gap::S, gap::M])
            .into(),
        )
    }

    /// For text too long for one post, the offer to split it; for a thread, each post
    /// as it will be sent.
    pub(super) fn thread_panel(&self, over: bool) -> Option<Element<'a, M>> {
        let c = self.composer;
        if !c.threaded() {
            return over.then(|| {
                let n = c.parts().len();
                row![
                    ui::muted("This is too long for one post.").width(Length::Fill),
                    ui::secondary(format!("Split into {n} posts"))
                        .on_press(self.msg(Msg::ToggleThread)),
                ]
                .spacing(gap::S)
                .align_y(Alignment::Center)
                .into()
            });
        }
        let parts = c.parts();
        let mut col = Column::new().spacing(gap::S).push(
            row![
                column![
                    text(format!("A thread of {} posts", parts.len())).size(size::BODY),
                    ui::faint(format!(
                        "Put {BREAK} on its own line to start the next post yourself."
                    )),
                ]
                .spacing(gap::XXS)
                .width(Length::Fill),
                ghost("Don't split", self.msg(Msg::ToggleThread)),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        );
        let last = parts.len().saturating_sub(1);
        for (i, part) in parts.into_iter().enumerate() {
            let images = c.images.iter().filter(|a| a.part.min(last) == i).count();
            let mut body = column![text(part).size(size::SMALL).line_height(1.5)].spacing(gap::XS);
            match images {
                0 => {}
                1 => body = body.push(ui::faint("With 1 image")),
                n => body = body.push(ui::faint(format!("With {n} images"))),
            }
            col = col.push(ui::card(body).padding([gap::S, gap::M]));
        }
        Some(col.into())
    }
}
