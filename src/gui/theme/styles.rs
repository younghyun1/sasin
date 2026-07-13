//! Widget style helpers, grounded against iced 0.14: `button::{primary,text}` provide
//! status-aware bases, and `extended_palette()` exposes the active theme's color groups.

use iced::widget::{button, container};
use iced::{Border, Color, Theme, border};

/// Style for the active tab / selected tree row: the filled primary accent.
pub fn selected(theme: &Theme, status: button::Status) -> button::Style {
    button::primary(theme, status)
}

/// Style for an inactive tab / unselected tree row: a flat text button (hover feedback only).
pub fn flat(theme: &Theme, status: button::Status) -> button::Style {
    button::text(theme, status)
}

/// A sidebar / panel container with a subtle background and border.
pub fn panel(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        border: Border {
            color: p.background.strong.color,
            width: 1.0,
            radius: border::radius(4.0),
        },
        ..container::Style::default()
    }
}

/// Accent color for an HTTP method badge.
pub fn method_color(method: &str, theme: &Theme) -> Color {
    let p = theme.extended_palette();
    match method.to_ascii_uppercase().as_str() {
        "GET" => p.success.base.color,
        "POST" => p.primary.base.color,
        "PUT" | "PATCH" => p.warning.base.color,
        "DELETE" => p.danger.base.color,
        _ => p.secondary.base.color,
    }
}
