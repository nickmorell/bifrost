//! The menu that sets what a feed leaves out.

use crate::components::{Icon, check_item, menu, menu_divider, menu_item, pill};
use crate::model::{FeedFilter, MIN_LIKES_STEPS};
use crate::theme::{self, gap, size};
use iced::widget::{Row, container, text};
use iced::{Element, Length};

/// The filters for one feed. Each choice sends the feed's filter as it would be with
/// that choice made.
pub fn filter_menu<'a>(f: FeedFilter) -> Element<'a, FeedFilter> {
    let mut likes = Row::new().spacing(gap::XS);
    for n in MIN_LIKES_STEPS {
        let label = if n == 0 {
            "Any".to_string()
        } else {
            n.to_string()
        };
        likes = likes.push(pill(
            label,
            f.min_likes == n,
            FeedFilter { min_likes: n, ..f },
        ));
    }
    let mut items = vec![
        check_item(
            "Hide reposts",
            f.hide_reposts,
            FeedFilter {
                hide_reposts: !f.hide_reposts,
                ..f
            },
        ),
        check_item(
            "Hide replies",
            f.hide_replies,
            FeedFilter {
                hide_replies: !f.hide_replies,
                ..f
            },
        ),
        check_item(
            "Hide quote posts",
            f.hide_quotes,
            FeedFilter {
                hide_quotes: !f.hide_quotes,
                ..f
            },
        ),
        check_item(
            "Only posts with media",
            f.media_only,
            FeedFilter {
                media_only: !f.media_only,
                ..f
            },
        ),
        menu_divider(),
        container(
            text("Minimum likes")
                .size(size::CAPTION)
                .style(theme::muted),
        )
        .padding([gap::XS, gap::M - 2.0])
        .into(),
        container(likes.wrap().vertical_spacing(gap::XS))
            .padding([0.0, gap::M - 2.0])
            .width(Length::Fill)
            .into(),
    ];
    if f.is_on() {
        items.push(menu_divider());
        items.push(menu_item(
            Icon::Close,
            "Clear filters",
            Some(FeedFilter::default()),
        ));
    }
    menu(items)
}
