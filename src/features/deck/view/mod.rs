//! Home as columns, each its own panel, scrolling sideways when they don't fit. What
//! a column shows is drawn by the app, so this builds the app's own message type `M`
//! through [`View::lift`].

mod add;

pub use add::AddColumn;

use super::message::Msg;
use super::state::{MAX_COLUMNS, scroll_id};
use crate::components::{self as ui, Icon, ghost, icon, icon_button};
use crate::model::{DeckColumn, DeckWidth};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{
    Button, Column, Row, button, center, column, container, row, scrollable, stack, text,
};
use iced::{Alignment, Element, Length, Padding};

/// What the deck needs from the app around it.
pub trait Host<'a, M> {
    /// What a column shows (its timeline, or the notifications), unscrolled.
    /// `scroll_id` names the column's scroll position.
    fn body(&self, column: &DeckColumn, scroll_id: &str) -> Element<'a, M>;
    /// How many new posts a background check found for a column.
    fn pending(&self, column: &DeckColumn) -> usize;
    /// The column's own buttons, before Refresh (its feed's filters).
    fn buttons(&self, column: &DeckColumn) -> Option<Element<'a, M>>;
}

/// The deck.
pub struct View<H, M> {
    pub columns: Vec<DeckColumn>,
    pub width: DeckWidth,
    /// Unread notifications, shown on the notifications column.
    pub unread: u64,
    pub host: H,
    /// Turns the deck's messages into the app's.
    pub lift: fn(Msg) -> M,
}

impl<'a, H: Host<'a, M>, M: Clone + 'a> View<H, M> {
    fn msg(&self, m: Msg) -> M {
        (self.lift)(m)
    }

    pub fn view(&self) -> Element<'a, M> {
        let cols = &self.columns;
        let width = self.width.pixels();
        let mut strip = Row::new().spacing(gap::S).height(Length::Fill);
        for (i, c) in cols.iter().enumerate() {
            strip = strip.push(self.column(i, c, cols.len(), width));
        }
        let mut end = Column::new()
            .spacing(gap::S)
            .align_x(Alignment::Center)
            .width(200)
            .height(Length::Fill);
        if cols.len() < MAX_COLUMNS {
            end = end.push(
                button(
                    center(
                        column![
                            icon(Icon::Plus, icon_size::XXL, Tone::Muted),
                            ui::muted("Add column")
                        ]
                        .spacing(gap::S)
                        .align_x(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::slot)
                .on_press(self.msg(Msg::OpenAdd)),
            );
        }
        end = end.push(ghost("One column", self.msg(Msg::Toggle(false))));
        strip = strip.push(end);
        // The scrollbar gets its own space under the columns instead of covering them.
        scrollable(strip)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .width(6)
                    .scroller_width(6)
                    .margin(2)
                    .spacing(gap::XS),
            ))
            .style(theme::scrollbar)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn column(&self, i: usize, c: &DeckColumn, count: usize, width: f32) -> Element<'a, M> {
        let mut title = row![
            icon(column_icon(c), icon_size::SM, Tone::Accent),
            text(c.title().to_string())
                .size(size::BODY)
                .font(theme::SEMIBOLD)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(gap::S)
        .align_y(Alignment::Center);
        if matches!(c, DeckColumn::Notifications) && self.unread > 0 {
            title = title.push(ui::count_badge(self.unread));
        }
        let mut header = Row::new()
            .spacing(gap::XXS)
            .align_y(Alignment::Center)
            .padding(Padding::from([gap::S, gap::S]).left(gap::M))
            .push(
                button(title)
                    .padding([gap::XS, 0.0])
                    .style(theme::bare)
                    .width(Length::Fill)
                    .on_press(self.msg(Msg::Top(i))),
            );
        if i > 0 {
            header = header.push(ui::tip(
                small(icon_button(Icon::ChevronLeft, self.msg(Msg::Move(i, -1)))),
                "Move left",
            ));
        }
        if i + 1 < count {
            header = header.push(ui::tip(
                small(icon_button(Icon::ChevronRight, self.msg(Msg::Move(i, 1)))),
                "Move right",
            ));
        }
        if let Some(b) = self.host.buttons(c) {
            header = header.push(b);
        }
        header = header
            .push(ui::tip(
                small(icon_button(Icon::Refresh, self.msg(Msg::Refresh(i)))),
                "Refresh",
            ))
            .push(ui::tip(
                small(icon_button(Icon::Close, self.msg(Msg::Remove(i)))),
                "Remove column",
            ));

        let id = scroll_id(c);
        let content = self.host.body(c, &id);
        let lift = self.lift;
        let body = ui::scroll(container(content).padding(Padding::ZERO.bottom(gap::XL)))
            .id(id)
            .on_scroll(move |v| lift(Msg::Scrolled(i, v)));
        let pending = self.host.pending(c);
        let body: Element<'a, M> = if pending > 0 {
            let label = if pending == 1 {
                "1 new post".to_string()
            } else {
                format!("{pending} new posts")
            };
            stack![
                body,
                container(ui::new_posts(label, self.msg(Msg::ShowNew(i))))
                    .padding(Padding::ZERO.top(gap::S))
                    .center_x(Length::Fill),
            ]
            .into()
        } else {
            body.into()
        };
        container(column![header, ui::divider_flush(), body])
            .style(theme::panel)
            .width(width)
            .height(Length::Fill)
            .clip(true)
            .into()
    }
}

fn column_icon(c: &DeckColumn) -> Icon {
    match c {
        DeckColumn::Following => Icon::Home,
        DeckColumn::Feed { .. } => Icon::Hash,
        DeckColumn::List { .. } => Icon::List,
        DeckColumn::Notifications => Icon::Bell,
        DeckColumn::Bookmarks => Icon::Bookmark,
        DeckColumn::Profile { .. } => Icon::User,
    }
}

/// A header button, a little smaller than usual.
fn small<M>(b: Button<'_, M>) -> Button<'_, M> {
    b.padding(gap::XS + 1.0)
}
