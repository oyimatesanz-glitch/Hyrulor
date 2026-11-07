use cosmic::iced::{Background, Color};
use cosmic::widget;

// Digital Aristocracy palette. All corners remain square in this first native pass.
pub fn forest() -> Color {
    Color::from_rgb(0.055, 0.090, 0.067)
}
pub fn card() -> Color {
    Color::from_rgb(0.090, 0.145, 0.110)
}
pub fn assistant_bubble() -> Color {
    Color::from_rgb(0.118, 0.165, 0.133)
}
pub fn user_bubble() -> Color {
    Color::from_rgb(0.078, 0.118, 0.090)
}
pub fn tool_bubble() -> Color {
    Color::from_rgb(0.160, 0.130, 0.082)
}
pub fn brass() -> Color {
    Color::from_rgb(0.690, 0.553, 0.341)
}
pub fn brass_dim() -> Color {
    Color::from_rgb(0.420, 0.330, 0.205)
}
pub fn champagne() -> Color {
    Color::from_rgb(0.850, 0.745, 0.525)
}
pub fn cream() -> Color {
    Color::from_rgb(0.910, 0.875, 0.785)
}
pub fn muted() -> Color {
    Color::from_rgb(0.600, 0.610, 0.565)
}
pub fn sage() -> Color {
    Color::from_rgb(0.478, 0.620, 0.494)
}
pub fn oxblood() -> Color {
    Color::from_rgb(0.355, 0.105, 0.145)
}
pub fn oxblood_light() -> Color {
    Color::from_rgb(0.720, 0.365, 0.390)
}
pub fn forest_dark() -> Color {
    Color::from_rgb(0.035, 0.055, 0.042)
}

pub fn container_style(color: Color) -> impl Fn(&cosmic::Theme) -> widget::container::Style {
    move |_| widget::container::Style {
        background: Some(Background::Color(color)),
        border: cosmic::iced::Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
