//! Who is live now: a card for each stream, the most watched first.

use super::{Host, View, viewers};
use crate::components::{self as ui, video_player::live_mark};
use crate::features::live::Msg;
use crate::model::LiveStream;
use crate::theme::{self, gap, size};
use iced::widget::{Column, Row, column, container, row, stack, text};
use iced::{Alignment, Element, Length};

/// How wide a stream's card is.
const CARD_W: f32 = 296.0;
/// How tall its picture is, at 16:9.
const THUMB_H: f32 = CARD_W * 9.0 / 16.0;

impl<'a, H: Host<M>, M: Clone + 'a> View<'a, H, M> {
    /// The streams live now.
    pub fn board(&self) -> Element<'a, M> {
        let b = &self.live.board;
        let body: Element<'a, M> = if b.loading && !b.loaded {
            ui::status("Loading…")
        } else if let (Some(e), true) = (&b.error, b.streams.is_empty()) {
            self.retry(e.clone(), Msg::RefreshBoard)
        } else if b.streams.is_empty() {
            ui::status("No one is streaming on Streamplace right now.")
        } else {
            let mut grid = Row::new().spacing(gap::L);
            for s in &b.streams {
                grid = grid.push(self.stream_card(s));
            }
            grid.wrap().vertical_spacing(gap::XL).into()
        };
        let top = column![
            text("Live now").size(size::TITLE).font(theme::SEMIBOLD),
            ui::muted("People streaming on Streamplace. Pick one to watch and chat."),
        ]
        .spacing(gap::XS);
        ui::scroll(
            Column::new()
                .push(top)
                .push(body)
                .spacing(gap::XL)
                .padding([gap::L, gap::XL])
                .width(Length::Fill),
        )
        .into()
    }

    fn stream_card(&self, s: &'a LiveStream) -> Element<'a, M> {
        let author = self.live.board.authors.get(&s.did);
        let name = author.map_or(s.handle.as_str(), |a| a.name());
        let mut marks = Row::new().spacing(gap::XS).push(live_mark());
        if let Some(n) = s.viewers {
            marks = marks.push(viewers(n));
        }
        let picture = stack![
            self.still(s.thumb.as_deref(), CARD_W, THUMB_H),
            container(marks).padding(gap::S),
        ];

        let mut info = Column::new().spacing(gap::XXS).width(Length::Fill).push(
            text(ui::truncate_lines(title_of(s), 2))
                .size(size::BODY)
                .font(theme::MEDIUM),
        );
        info = info.push(ui::faint(name).size(size::SMALL));
        if let Some(a) = &s.activity {
            info = info.push(ui::faint(a.as_str()).size(size::SMALL));
        }
        let about = row![
            self.avatar(author.and_then(|a| a.avatar.as_deref()), 32.0),
            info
        ]
        .spacing(gap::S)
        .align_y(Alignment::Start);

        ui::hand(column![picture, about].spacing(gap::S).width(CARD_W))
            .on_press(self.msg(Msg::OpenStream(actor_of(s))))
            .into()
    }
}

/// What a stream is called, or a stand-in for streams without a title.
pub(super) fn title_of(s: &LiveStream) -> &str {
    if s.title.trim().is_empty() {
        "Untitled stream"
    } else {
        &s.title
    }
}

/// How to open a streamer's page: by handle, or by DID when the handle is broken.
fn actor_of(s: &LiveStream) -> String {
    if s.handle.is_empty() || s.handle == "handle.invalid" {
        s.did.to_string()
    } else {
        s.handle.clone()
    }
}
