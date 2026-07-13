//! Divider styling contract for [`super::Split`]. The actual colors live in
//! `gui::theme::styles` (the project's centralized styling), which implements [`Catalog`]
//! for `iced::Theme`.

use iced::Color;

/// Interaction state of the divider, from the widget's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerStatus {
    Idle,
    Hovered,
    Dragging,
}

/// Resolved divider appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DividerStyle {
    pub color: Color,
    /// Drawn width in pixels (may exceed the 1px layout gap; the strip is centered on it).
    pub width: f32,
}

/// Theme hook: maps a [`DividerStatus`] to a [`DividerStyle`].
pub trait Catalog {
    fn divider(&self, status: DividerStatus) -> DividerStyle;
}
