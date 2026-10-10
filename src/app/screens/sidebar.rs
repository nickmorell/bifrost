//! The sidebar: the screens, your account and the composer button.

use crate::app::util::avatar;
use crate::app::{App, Message, ProfileTab, Route, compose, nav};
use crate::components::{self as ui, Icon, icon, nav_item};
use crate::theme::icon_size;
use crate::theme::{self, Tone, gap, layout, size};
use iced::widget::{button, column, container, row, space, text};
use iced::{Alignment, Element, Length};

impl App {
    pub(super) fn sidebar(&self) -> Element<'_, Message> {
        let current = self.routes.first().cloned().unwrap_or(Route::Home);
        let rail = self.rail();
        let nav = |i: Icon, label: &'static str, route: Route, badge: Option<u64>| {
            nav_item(
                i,
                label,
                current == route,
                badge,
                Message::Nav(nav::Msg::Navigate(route)),
                rail,
            )
        };

        let brand: Element<'_, Message> = if rail {
            container(ui::brand_mark(icon_size::XXL))
                .center_x(Length::Fill)
                .into()
        } else {
            row![
                ui::brand_mark(icon_size::XXL),
                text("BiFrost")
                    .size(size::LARGE)
                    .font(theme::SEMIBOLD)
                    .style(theme::muted),
            ]
            .spacing(gap::S + 2.0)
            .align_y(Alignment::Center)
            .into()
        };
        let compose: Element<'_, Message> = if rail {
            ui::tip(
                button(
                    container(icon(Icon::Compose, icon_size::M, Tone::OnAccent))
                        .center_x(Length::Fill),
                )
                .width(Length::Fill)
                .padding([gap::S + 2.0, 0.0])
                .style(theme::primary)
                .on_press(Message::Compose(compose::Msg::New)),
                "New post",
            )
        } else {
            ui::primary_icon(Icon::Compose, "New post")
                .width(Length::Fill)
                .padding([gap::S + 2.0, gap::M])
                .on_press(Message::Compose(compose::Msg::New))
                .into()
        };

        let mut col = column![
            container(brand).padding([gap::S, if rail { 0.0 } else { gap::M }]),
            space::vertical().height(gap::M),
            nav(Icon::Home, "Home", Route::Home, None),
            nav(
                Icon::Bell,
                "Notifications",
                Route::Notifications,
                Some(self.notifications.unread())
            ),
            nav(Icon::Chat, "Chats", Route::Chats, Some(self.chats.unread)),
            if self.streamplace_on() {
                nav(Icon::Live, "Live", Route::Live, None)
            } else {
                space::vertical().height(0).into()
            },
            nav(Icon::Search, "Search", Route::Search, None),
            nav(Icon::Hash, "Feeds", Route::Feeds, None),
            nav(Icon::List, "Lists", Route::Lists, None),
            nav(Icon::Article, "Articles", Route::Articles, None),
            nav(
                Icon::User,
                "Profile",
                Route::Profile(self.my_did.to_string(), ProfileTab::Posts),
                None
            ),
            nav(Icon::Bookmark, "Bookmarks", Route::Bookmarks, None),
            nav(
                Icon::Drafts,
                "Drafts",
                Route::Drafts,
                Some(self.drafts.list.len() as u64)
            ),
            nav(Icon::Settings, "Settings", Route::Settings, None),
            space::vertical().height(gap::L),
            compose,
            space::vertical(),
        ]
        .spacing(gap::XXS)
        .padding([gap::L, if rail { gap::S } else { gap::M }])
        .width(if rail { layout::RAIL } else { layout::SIDEBAR })
        .height(Length::Fill);

        if let Some(me) = &self.me {
            let mut who = row![avatar(self, me.avatar.as_deref(), 30.0)]
                .spacing(gap::S + 2.0)
                .align_y(Alignment::Center);
            if !rail {
                who = who.push(
                    column![
                        text(me.name()).size(size::SMALL).font(theme::MEDIUM),
                        ui::faint(format!("@{}", me.handle)).size(size::CAPTION),
                    ]
                    .spacing(gap::HAIR)
                    .width(Length::Fill),
                );
            }
            col = col.push(
                button(container(who).center_x(Length::Fill))
                    .width(Length::Fill)
                    .padding(gap::S)
                    .style(theme::row_button)
                    .on_press(Message::Nav(nav::Msg::OpenProfile(me.did.to_string()))),
            );
        }
        col.into()
    }
}
