//! Lucide icon font (v1.24.0, ISC license). Codepoints are taken verbatim from the release's
//! `info.json`; never guess them. Icons render as text glyphs, so they inherit the text color
//! of their context and theme for free.
//!
//! Full vendored codepoint map for reference (add a `const` when a glyph gets used):
//!   chevron-right e06f  chevron-down e06d  x e1b2         check e06c      circle-x e084
//!   play e13c           send e152          save e14d      copy e09e       trash-2 e18e
//!   plus e13d           folder e0d7        folder-plus e0d9  globe e0e8   history e1f5
//!   cookie e26b         search e151        sun e178        moon e11e      triangle-alert e193
//!   dot e44f            pencil e1f9        file-down e318  arrow-left e048  arrow-right e049
//!   circle e076         settings e154      clipboard-copy e225  send-horizontal e4f2

use iced::Font;
use iced::widget::{Text, text};

/// The registered icon font (family name inside the TTF is "lucide").
pub const FONT: Font = Font::with_name("lucide");

pub const BYTES: &[u8] = include_bytes!("../../../assets/fonts/lucide.ttf");

pub const SUN: char = '\u{e178}';
pub const MOON: char = '\u{e11e}';
pub const CHEVRON_RIGHT: char = '\u{e06f}';
pub const CHEVRON_DOWN: char = '\u{e06d}';
pub const X: char = '\u{e1b2}';
pub const CHECK: char = '\u{e06c}';
pub const CIRCLE_X: char = '\u{e084}';
pub const PLAY: char = '\u{e13c}';
pub const TRASH: char = '\u{e18e}';
pub const ARROW_LEFT: char = '\u{e048}';
pub const ARROW_RIGHT: char = '\u{e049}';
pub const ARROW_UP: char = '\u{e04a}';
pub const ARROW_DOWN: char = '\u{e042}';
pub const DOT: char = '\u{e44f}';
pub const PENCIL: char = '\u{e1f9}';
pub const COPY: char = '\u{e09e}';
pub const PLUS: char = '\u{e13d}';
pub const FOLDER_PLUS: char = '\u{e0d9}';

/// A single icon glyph as a text widget; color/style comes from the surrounding context.
pub fn icon<'a>(glyph: char, size: f32) -> Text<'a> {
    text(glyph.to_string()).font(FONT).size(size)
}
