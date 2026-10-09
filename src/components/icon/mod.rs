//! Interface icons, tinted with a theme [`Tone`].

mod set;

pub use set::Icon;

use crate::theme::{self, Tone};
use iced::Element;
use iced::widget::svg;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Icon handles are created once so iced can cache their rasterised images.
static ICON_CACHE: LazyLock<Mutex<HashMap<Icon, svg::Handle>>> = LazyLock::new(Default::default);

fn handle(icon: Icon) -> svg::Handle {
    let mut cache = ICON_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry(icon)
        .or_insert_with(|| svg::Handle::from_memory(icon.svg().into_bytes()))
        .clone()
}

/// An icon tinted with `tint`.
pub fn icon<'a, M: 'a>(icon: Icon, px: f32, tint: Tone) -> Element<'a, M> {
    svg(handle(icon))
        .width(px)
        .height(px)
        .style(theme::icon(tint))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_are_white_svg_documents() {
        let doc = Icon::Heart.svg();
        assert!(doc.starts_with("<svg") && doc.contains("#ffffff"));
        // The same icon reuses one handle, so iced rasterises it once.
        assert_eq!(handle(Icon::Home), handle(Icon::Home));
    }
}
