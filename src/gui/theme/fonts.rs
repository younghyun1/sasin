//! Embedded application fonts: Inter for UI text, JetBrains Mono for code surfaces.
//! Family-name constants must match the fonts' internal names (verified with `fc-scan`).

use iced::Font;
use iced::font::{Family, Weight};

/// Default UI font (registered in `main` via `.default_font`).
pub const UI: Font = Font::with_name("Inter");

/// Semibold UI variant for emphasis (method badges, headers).
pub const UI_SEMIBOLD: Font = Font {
    family: Family::Name("Inter"),
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

/// Monospace font for editors, response bodies, and transcripts.
pub const MONO: Font = Font::with_name("JetBrains Mono");

pub const BYTES_UI: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.otf");
pub const BYTES_UI_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.otf");
pub const BYTES_MONO: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");
