//! Text fields, pickers, editors and switches.

use super::{alpha, soft_shadow};
use crate::theme::palette;
use crate::theme::tokens::{Palette, radius};
use iced::overlay::menu;
use iced::widget::{pick_list, text_editor, text_input, toggler};
use iced::{Border, Color, Theme};

fn input_border(p: &Palette, focused: bool, hovered: bool) -> Border {
    Border {
        color: if focused {
            alpha(p.accent, 0.55)
        } else if hovered {
            p.pressed
        } else {
            p.hairline
        },
        width: 1.0,
        radius: radius::M.into(),
    }
}

pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let p = palette(theme);
    let (focused, hovered) = match status {
        text_input::Status::Focused { .. } => (true, false),
        text_input::Status::Hovered => (false, true),
        _ => (false, false),
    };
    text_input::Style {
        background: p.raised.into(),
        border: input_border(p, focused, hovered),
        icon: p.faint,
        placeholder: p.faint,
        value: p.text,
        selection: alpha(p.accent, 0.35),
    }
}

pub fn select(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let p = palette(theme);
    let (open, hovered) = match status {
        pick_list::Status::Opened { .. } => (true, false),
        pick_list::Status::Hovered => (false, true),
        pick_list::Status::Active => (false, false),
    };
    pick_list::Style {
        text_color: p.text,
        placeholder_color: p.muted,
        handle_color: p.muted,
        background: p.raised.into(),
        border: input_border(p, open, hovered),
    }
}

pub fn select_menu(theme: &Theme) -> menu::Style {
    let p = palette(theme);
    menu::Style {
        background: p.overlay.into(),
        border: Border {
            color: p.hairline,
            width: 1.0,
            radius: radius::M.into(),
        },
        text_color: p.text,
        selected_text_color: p.on_accent,
        selected_background: p.accent.into(),
        shadow: soft_shadow(28.0, 8.0, 0.45),
    }
}

pub fn editor(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let p = palette(theme);
    let (focused, hovered) = match status {
        text_editor::Status::Focused { .. } => (true, false),
        text_editor::Status::Hovered => (false, true),
        _ => (false, false),
    };
    text_editor::Style {
        background: p.raised.into(),
        border: input_border(p, focused, hovered),
        placeholder: p.faint,
        value: p.text,
        selection: alpha(p.accent, 0.35),
    }
}

pub fn switch(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let p = palette(theme);
    let (on, hovered) = match status {
        toggler::Status::Active { is_toggled } => (is_toggled, false),
        toggler::Status::Hovered { is_toggled } => (is_toggled, true),
        toggler::Status::Disabled { is_toggled } => (is_toggled, false),
    };
    toggler::Style {
        background: if on {
            p.accent.into()
        } else if hovered {
            p.overlay.into()
        } else {
            p.raised.into()
        },
        background_border_width: 1.0,
        background_border_color: if on { p.accent } else { p.hairline },
        foreground: if on {
            p.on_accent.into()
        } else {
            p.muted.into()
        },
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: Some(p.text),
        border_radius: None,
        padding_ratio: 0.18,
    }
}
