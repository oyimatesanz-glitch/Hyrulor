//! Design tokens for the Hyrulor popup.
//!
//! Palette taken from `/home/oyi/Proyectos/designs/index.html`:
//! obsidian backgrounds, gold framing, cyan for tool/system data and glass
//! surfaces with generously rounded corners.

use cosmic::iced::{Background, Border, Color, Shadow, Vector};
use cosmic::widget;

// --- Backgrounds -----------------------------------------------------------

pub fn background() -> Color {
    Color::from_rgb8(0x05, 0x05, 0x05)
}

pub fn glass() -> Color {
    Color::from_rgba8(0x0A, 0x0A, 0x0A, 0.82)
}

pub fn card() -> Color {
    Color::from_rgba8(0x14, 0x14, 0x14, 0.72)
}

pub fn assistant_bubble() -> Color {
    Color::from_rgba8(0x14, 0x14, 0x14, 0.68)
}

pub fn user_bubble() -> Color {
    Color::from_rgba8(0xD4, 0xAF, 0x37, 0.08)
}

pub fn tool_bubble() -> Color {
    Color::from_rgba8(0x00, 0xD4, 0xFF, 0.07)
}

// --- Accents ---------------------------------------------------------------

pub fn gold() -> Color {
    Color::from_rgb8(0xD4, 0xAF, 0x37)
}

pub fn gold_light() -> Color {
    Color::from_rgb8(0xFF, 0xD7, 0x00)
}

pub fn gold_dark() -> Color {
    Color::from_rgb8(0x8B, 0x69, 0x14)
}

pub fn cyan() -> Color {
    Color::from_rgb8(0x00, 0xD4, 0xFF)
}

pub fn success() -> Color {
    Color::from_rgb8(0x00, 0xFF, 0x88)
}

pub fn danger() -> Color {
    Color::from_rgb8(0xFF, 0x44, 0x44)
}

// --- Text ------------------------------------------------------------------

pub fn text_primary() -> Color {
    Color::from_rgb8(0xF8, 0xF9, 0xFA)
}

pub fn text_secondary() -> Color {
    Color::from_rgb8(0x9C, 0xA3, 0xAF)
}

pub fn text_muted() -> Color {
    Color::from_rgb8(0x6B, 0x72, 0x80)
}

// --- Borders ---------------------------------------------------------------

pub fn border_subtle() -> Color {
    Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.08)
}

pub fn border_gold() -> Color {
    Color::from_rgba8(0xD4, 0xAF, 0x37, 0.30)
}

pub fn border_cyan() -> Color {
    Color::from_rgba8(0x00, 0xD4, 0xFF, 0.28)
}

/// Text colour class used by `widget::text(..).class(..)`.
pub fn text_color(color: Color) -> cosmic::theme::Text {
    cosmic::theme::Text::Color(color)
}

// --- Surfaces --------------------------------------------------------------

fn container_style(
    color: Color,
    border: Color,
    width: f32,
    radius: f32,
    text: Color,
) -> widget::container::Style {
    widget::container::Style {
        text_color: Some(text),
        background: Some(Background::Color(color)),
        border: Border {
            color: border,
            width,
            radius: radius.into(),
        },
        ..Default::default()
    }
}

/// Rounded surface without a visible frame.
pub fn surface_radius(
    color: Color,
    radius: f32,
) -> impl Fn(&cosmic::Theme) -> widget::container::Style {
    move |_| container_style(color, Color::TRANSPARENT, 0.0, radius, text_primary())
}

/// Glass/panel surface with a one-pixel frame.
pub fn surface_framed(
    color: Color,
    border: Color,
    radius: f32,
) -> impl Fn(&cosmic::Theme) -> widget::container::Style {
    move |_| container_style(color, border, 1.0, radius, text_primary())
}

// --- Buttons ---------------------------------------------------------------

fn button_style(
    background: Color,
    text: Color,
    border: Color,
    radius: f32,
    glow: bool,
) -> cosmic::iced::widget::button::Style {
    let shadow = if glow {
        Shadow {
            color: Color::from_rgba8(0xD4, 0xAF, 0x37, 0.35),
            offset: Vector::new(0.0, 0.0),
            blur_radius: 18.0,
        }
    } else {
        Shadow::default()
    };

    cosmic::iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: text,
        icon_color: Some(text),
        border_radius: radius.into(),
        border_width: if border == Color::TRANSPARENT {
            0.0
        } else {
            1.0
        },
        border_color: border,
        border: Border {
            color: border,
            width: if border == Color::TRANSPARENT {
                0.0
            } else {
                1.0
            },
            radius: radius.into(),
        },
        shadow,
        ..Default::default()
    }
}

/// Solid gold pill used for the primary action (send/authorize).
pub fn primary_button() -> cosmic::theme::iced::Button {
    use cosmic::iced::widget::button::Status;

    cosmic::theme::iced::Button::Custom(Box::new(|_, status| match status {
        Status::Hovered => {
            button_style(gold_light(), background(), Color::TRANSPARENT, 100.0, true)
        }
        Status::Pressed => {
            button_style(gold_dark(), background(), Color::TRANSPARENT, 100.0, false)
        }
        Status::Disabled => {
            button_style(gold_dark(), text_muted(), Color::TRANSPARENT, 100.0, false)
        }
        Status::Active => button_style(gold(), background(), Color::TRANSPARENT, 100.0, true),
    }))
}

/// Transparent gold-outlined pill used for secondary actions.
pub fn outline_button() -> cosmic::theme::iced::Button {
    use cosmic::iced::widget::button::Status;

    cosmic::theme::iced::Button::Custom(Box::new(|_, status| match status {
        Status::Hovered | Status::Pressed => button_style(
            Color::from_rgba8(0xD4, 0xAF, 0x37, 0.20),
            gold_light(),
            border_gold(),
            100.0,
            false,
        ),
        Status::Disabled => button_style(
            Color::TRANSPARENT,
            text_muted(),
            border_subtle(),
            100.0,
            false,
        ),
        Status::Active => button_style(
            Color::from_rgba8(0xD4, 0xAF, 0x37, 0.10),
            gold(),
            border_gold(),
            100.0,
            false,
        ),
    }))
}

/// Borderless button used for low-emphasis actions such as "Cancelar".
pub fn ghost_button() -> cosmic::theme::iced::Button {
    use cosmic::iced::widget::button::Status;

    cosmic::theme::iced::Button::Custom(Box::new(|_, status| match status {
        Status::Hovered | Status::Pressed => button_style(
            Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.06),
            text_primary(),
            Color::TRANSPARENT,
            100.0,
            false,
        ),
        Status::Disabled => button_style(
            Color::TRANSPARENT,
            text_muted(),
            Color::TRANSPARENT,
            100.0,
            false,
        ),
        Status::Active => button_style(
            Color::TRANSPARENT,
            text_secondary(),
            Color::TRANSPARENT,
            100.0,
            false,
        ),
    }))
}
