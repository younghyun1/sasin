//! Custom dark/light palettes and the theme constructor.
//!
//! Both themes share the same accent (`#ff6c37`); surfaces layer via hand-tuned
//! `background.weak`/`strong` pairs instead of the generated ramp so panels and hairlines
//! read like a desktop tool rather than derived tints.

use std::sync::LazyLock;

use iced::theme::palette::Extended;
use iced::theme::{Palette, palette};
use iced::{Color, Theme};

use crate::persist::ThemeChoice;

/// The active [`Theme`] for a persisted [`ThemeChoice`]. Cheap: clones an `Arc`.
pub fn app_theme(choice: ThemeChoice) -> Theme {
    match choice {
        ThemeChoice::Dark => DARK.clone(),
        ThemeChoice::Light => LIGHT.clone(),
    }
}

/// Syntax-highlight theme matching the app theme. `InspiredGitHub` is the only light theme
/// iced's highlighter ships; `Base16Ocean` is the most neutral of its dark options.
pub fn code_theme(choice: ThemeChoice) -> iced::highlighter::Theme {
    match choice {
        ThemeChoice::Dark => iced::highlighter::Theme::Base16Ocean,
        ThemeChoice::Light => iced::highlighter::Theme::InspiredGitHub,
    }
}

static DARK: LazyLock<Theme> = LazyLock::new(|| {
    let base = Palette {
        background: hex(0x1e1e1e),
        text: hex(0xf2f2f1),
        primary: hex(0xff6c37),
        success: hex(0x6bdd9a),
        warning: hex(0xffe47e),
        danger: hex(0xff7a72),
    };
    Theme::custom_with_fn("sasin dark", base, |p| {
        let mut ext = Extended::generate(p);
        ext.background.weak = pair(0x262626, p.text);
        ext.background.strong = pair(0x383838, p.text);
        ext
    })
});

static LIGHT: LazyLock<Theme> = LazyLock::new(|| {
    let base = Palette {
        background: hex(0xffffff),
        text: hex(0x212121),
        primary: hex(0xff6c37),
        success: hex(0x007f31),
        warning: hex(0xa37f22),
        danger: hex(0xb02a1c),
    };
    Theme::custom_with_fn("sasin light", base, |p| {
        let mut ext = Extended::generate(p);
        ext.background.weak = pair(0xf7f7f7, p.text);
        ext.background.strong = pair(0xe0e0e0, p.text);
        ext
    })
});

fn hex(rgb: u32) -> Color {
    Color::from_rgb8(
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    )
}

fn pair(rgb: u32, text: Color) -> palette::Pair {
    palette::Pair::new(hex(rgb), text)
}
