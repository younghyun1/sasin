//! Widget style helpers, grounded against iced 0.14: `button::{primary,text}` provide
//! status-aware bases, and `extended_palette()` exposes the active theme's color groups.

use iced::widget::{button, container};
use iced::{Border, Color, Theme, border};

use crate::gui::components::split;

/// Style for the active tab / selected tree row: the filled primary accent.
pub fn selected(theme: &Theme, status: button::Status) -> button::Style {
    button::primary(theme, status)
}

/// Style for an inactive tab / unselected tree row: a flat text button (hover feedback only).
pub fn flat(theme: &Theme, status: button::Status) -> button::Style {
    button::text(theme, status)
}

/// Selected tree row: a soft accent fill instead of the loud primary button.
pub fn tree_row_selected(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    button::Style {
        background: Some(p.primary.weak.color.into()),
        text_color: p.primary.weak.text,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(4.0),
        },
        ..button::text(theme, status)
    }
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

/// Split-divider styling: a hairline at rest, an accent strip on hover/drag.
impl split::Catalog for Theme {
    fn divider(&self, status: split::DividerStatus) -> split::DividerStyle {
        let p = self.extended_palette();
        match status {
            split::DividerStatus::Idle => split::DividerStyle {
                color: p.background.strong.color,
                width: 1.0,
            },
            split::DividerStatus::Hovered => split::DividerStyle {
                color: Color {
                    a: 0.6,
                    ..p.primary.base.color
                },
                width: 3.0,
            },
            split::DividerStatus::Dragging => split::DividerStyle {
                color: p.primary.base.color,
                width: 3.0,
            },
        }
    }
}

/// Accent color for an HTTP method badge (Postman-style semantics: GET green, POST amber,
/// PUT blue, PATCH violet, DELETE red). Blue/violet are hand-tuned per theme because the
/// six-slot palette has no slots for them.
pub fn method_color(method: &str, theme: &Theme) -> Color {
    let p = theme.extended_palette();
    match method.to_ascii_uppercase().as_str() {
        "GET" => p.success.base.color,
        "POST" => p.warning.base.color,
        "PUT" => {
            if p.is_dark {
                super::palette::hex(0x74aef6)
            } else {
                super::palette::hex(0x0053b8)
            }
        }
        "PATCH" => {
            if p.is_dark {
                super::palette::hex(0xc0a8e1)
            } else {
                super::palette::hex(0x623497)
            }
        }
        "DELETE" => p.danger.base.color,
        _ => p.secondary.base.color,
    }
}

/// Color for an HTTP status class: 2xx success, 3xx accent, 4xx warning, 5xx danger.
pub fn status_color(code: u16, theme: &Theme) -> Color {
    let p = theme.extended_palette();
    match code / 100 {
        2 => p.success.base.color,
        3 => p.primary.base.color,
        4 => p.warning.base.color,
        5 => p.danger.base.color,
        _ => p.secondary.base.color,
    }
}

/// Muted, secondary text (section headers, meta chips).
pub fn muted(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.extended_palette().secondary.base.color),
    }
}

/// The application's base surface (root container background).
pub fn surface(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        ..container::Style::default()
    }
}

/// The bottom status-bar strip: a weak surface that reads as chrome, not content.
pub fn status_bar(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        border: Border {
            color: p.background.strong.color,
            width: 1.0,
            radius: border::radius(0.0),
        },
        ..container::Style::default()
    }
}

/// A rounded pill tinted by the response's status class; pair with text in `status_color`.
pub fn pill_for_status(code: u16) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let color = status_color(code, theme);
        container::Style {
            background: Some(Color { a: 0.15, ..color }.into()),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: border::radius(8.0),
            },
            ..container::Style::default()
        }
    }
}

/// The underline rule below a tab: accent when active, transparent otherwise.
pub fn underline(active: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| container::Style {
        background: Some(if active {
            theme.extended_palette().primary.base.color.into()
        } else {
            Color::TRANSPARENT.into()
        }),
        ..container::Style::default()
    }
}
