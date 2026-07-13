//! Centralized styling per the project convention: palettes, fonts, icons, style helpers.

pub mod fonts;
pub mod icons;
mod palette;
mod styles;

pub use palette::{app_theme, code_theme};
pub use styles::{flat, method_color, panel, selected};
