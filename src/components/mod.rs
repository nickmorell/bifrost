//! BiFrost's component kit: the building blocks every screen is made of.
//!
//! A component takes plain data and the messages it may send, and returns UI styled
//! from [`crate::theme`]. It never fetches, stores, or knows which screen it is on,
//! and works with any message type, so each one is tested on its own with
//! `iced_test`. One folder per component. Small widgets are re-exported here; the
//! larger ones (`media`, `post_card`, `video_player`) are used through their module.

mod avatar;
mod badge;
mod brand;
mod button;
mod feedback;
mod footed;
mod icon;
mod input;
mod lazy;
mod link_card;
pub mod media;
mod menu;
mod nav;
mod popover;
pub mod post_card;
mod surface;
mod tabs;
mod text;
pub mod video_player;

pub use avatar::{Frames, Picture, animation_fit, avatar, feed_avatar, live_avatar, mark, picture};
pub use badge::verified;
pub use brand::brand_mark;
pub use button::{
    action, danger, danger_icon_action, ghost, hand, icon_action, icon_button, icon_toggle,
    new_posts, primary, primary_icon, secondary,
};
pub use feedback::{retry, sentence, status, tip, toast};
pub use footed::footed;
pub use icon::{Icon, icon};
pub use input::{field, select};
pub use lazy::{Heights, Window, lazy_list, row_key};
pub use link_card::{
    Art, ArticleCard, Follow, Integration, Link, article_card, integration_card, link_card,
    record_link,
};
pub use menu::{check_item, danger_item, menu, menu_divider, menu_item};
pub use nav::{count_badge, nav_item};
pub use popover::popover;
pub use surface::{card, divider, divider_flush, marker, outlined, reading, scroll};
pub use tabs::{pill, tab_strip};
pub use text::{
    compact, eyebrow, faint, full_time, muted, relative_time, truncate, truncate_lines,
};
