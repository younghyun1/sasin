//! Centralized styling per the project convention: palettes, fonts, icons, style helpers.

pub mod fonts;
pub mod icons;
pub mod metrics;
mod palette;
mod styles;

pub use palette::{app_theme, code_theme};
pub use styles::{
    flat, method_color, muted, panel, pill_for_status, selected, status_bar, status_color, surface,
    tree_row_selected, underline,
};
