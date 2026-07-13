//! Centralized styling per the project convention: palettes, style helpers.

mod palette;
mod styles;

pub use palette::app_theme;
pub use styles::{flat, method_color, panel, selected};
