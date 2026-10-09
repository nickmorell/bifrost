//! Choosing a new column.

use super::column_icon;
use crate::components::{self as ui, Icon, ghost, icon, icon_button, pill};
use crate::features::deck::Msg;
use crate::model::{DeckColumn, DeckWidth, Did, FeedSource, ListInfo};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, size};
use iced::widget::{Column, Row, button, column, container, row, space, text};
use iced::{Alignment, Element, Length};

/// The "Add a column" dialog.
pub struct AddColumn<'a> {
    /// The columns there are now, which aren't offered again.
    pub have: Vec<DeckColumn>,
    /// The account signed in, for a column of its own posts.
    pub me: Option<&'a Did>,
    /// The feeds and lists pinned to Home.
    pub pinned: Vec<(&'a FeedSource, &'a str)>,
    /// Your lists.
    pub lists: &'a [ListInfo],
    pub width: DeckWidth,
}

impl<'a> AddColumn<'a> {
    pub fn view(&self) -> Element<'a, Msg> {
        let have = &self.have;
        let option = |c: DeckColumn, hint: &'static str| -> Option<Element<'a, Msg>> {
            if have.contains(&c) {
                return None;
            }
            Some(
                button(
                    row![
                        container(icon(column_icon(&c), icon_size::M, Tone::Accent))
                            .center_x(32)
                            .center_y(32)
                            .style(theme::raised),
                        column![
                            text(c.title().to_string())
                                .size(size::BODY)
                                .font(theme::MEDIUM),
                            ui::faint(hint).size(size::CAPTION),
                        ]
                        .spacing(gap::XXS),
                    ]
                    .spacing(gap::M)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([gap::S, gap::S])
                .style(theme::row_button)
                .on_press(Msg::Add(c))
                .into(),
            )
        };
        let mut col = Column::new().spacing(gap::XXS);
        let mut general = vec![
            option(DeckColumn::Following, "Posts from people you follow"),
            option(
                DeckColumn::Notifications,
                "Likes, replies, follows and more",
            ),
            option(DeckColumn::Bookmarks, "Posts you saved"),
        ];
        if let Some(did) = self.me {
            general.push(option(
                DeckColumn::Profile {
                    did: did.clone(),
                    name: "Your posts".into(),
                },
                "What you've posted",
            ));
        }
        for o in general.into_iter().flatten() {
            col = col.push(o);
        }
        let feeds: Vec<_> = self
            .pinned
            .iter()
            .filter(|(source, _)| **source != FeedSource::Following)
            .filter_map(|(source, name)| {
                option(
                    DeckColumn::for_source(source, name),
                    match source {
                        FeedSource::List(_) => "Pinned list",
                        _ => "Pinned feed",
                    },
                )
            })
            .collect();
        if !feeds.is_empty() {
            col = col.push(container(ui::eyebrow("Pinned feeds")).padding([gap::S, gap::S]));
            for f in feeds {
                col = col.push(f);
            }
        }
        let lists: Vec<_> = self
            .lists
            .iter()
            .filter(|l| l.purpose.ends_with("curatelist"))
            .filter_map(|l| {
                option(
                    DeckColumn::List {
                        uri: l.uri.clone(),
                        name: l.name.clone(),
                    },
                    "Your list",
                )
            })
            .collect();
        if !lists.is_empty() {
            col = col.push(container(ui::eyebrow("Your lists")).padding([gap::S, gap::S]));
            for l in lists {
                col = col.push(l);
            }
        }
        let mut widths = Row::new().spacing(gap::XS);
        for w in DeckWidth::ALL {
            widths = widths.push(pill(w.label(), self.width == w, Msg::Width(w)));
        }
        container(
            column![
                row![
                    text("Add a column").size(size::LARGE).font(theme::SEMIBOLD),
                    space::horizontal(),
                    icon_button(Icon::Close, Msg::CloseAdd),
                ]
                .align_y(Alignment::Center),
                ui::muted("Other feeds and lists can be added from their own pages."),
                ui::scroll(col).height(Length::Shrink),
                row![ui::faint("Column width"), space::horizontal(), widths]
                    .align_y(Alignment::Center),
                row![
                    ghost("Use one column", Msg::Toggle(false)),
                    space::horizontal(),
                    ui::secondary("Done").on_press(Msg::CloseAdd),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(gap::M),
        )
        .padding(gap::L)
        .width(theme::layout::DIALOG)
        .max_height(620)
        .style(theme::dialog)
        .into()
    }
}
