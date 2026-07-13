//! App-level UI preferences, persisted as human-readable TOML in the state directory
//! (`config.toml`, next to the workspace). Unlike the workspace this is per-machine state:
//! window geometry, pane layout, theme choice.
//!
//! Loading is infallible: a missing or unreadable file yields defaults, and every field is
//! `#[serde(default)]` so partial or older files never fail to parse.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::persist::paths::app_state_dir;
use crate::storage::io_util::write_atomic;

/// Which of the two built-in palettes the app renders with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    Dark,
    Light,
}

impl ThemeChoice {
    /// The other choice (used by the toggle).
    pub fn flipped(self) -> Self {
        match self {
            ThemeChoice::Dark => ThemeChoice::Light,
            ThemeChoice::Light => ThemeChoice::Dark,
        }
    }
}

/// Persisted window geometry. On Wayland the compositor owns placement, so only
/// size + maximized are stored; there is no position field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowPrefs {
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
}

impl Default for WindowPrefs {
    fn default() -> Self {
        Self {
            width: 1280.0,
            height: 800.0,
            maximized: false,
        }
    }
}

/// Persisted split-pane positions (pixels of the first pane).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutPrefs {
    pub sidebar_px: f32,
    pub editor_px: f32,
}

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self {
            sidebar_px: 300.0,
            editor_px: 360.0,
        }
    }
}

/// The whole preferences file.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPrefs {
    pub theme: ThemeChoice,
    pub window: WindowPrefs,
    pub layout: LayoutPrefs,
}

impl UiPrefs {
    /// Window size as an [`iced::Size`], clamped to sane bounds.
    pub fn window_size(&self) -> iced::Size {
        iced::Size::new(self.window.width, self.window.height)
    }

    /// Clamp every numeric field to sane, finite bounds. Guards against a hand-edited or
    /// corrupted file (TOML happily round-trips `nan` / absurd values).
    fn sanitized(mut self) -> Self {
        self.window.width = clamp_finite(self.window.width, 1280.0, 600.0, 8192.0);
        self.window.height = clamp_finite(self.window.height, 800.0, 400.0, 8192.0);
        self.layout.sidebar_px = clamp_finite(self.layout.sidebar_px, 300.0, 220.0, 560.0);
        self.layout.editor_px = clamp_finite(self.layout.editor_px, 360.0, 220.0, 900.0);
        self
    }
}

fn clamp_finite(v: f32, default: f32, min: f32, max: f32) -> f32 {
    if v.is_finite() {
        v.clamp(min, max)
    } else {
        default
    }
}

fn config_path() -> PathBuf {
    app_state_dir().join("config.toml")
}

/// Load preferences, falling back to defaults on any error (missing file is not logged).
pub fn load_prefs() -> UiPrefs {
    let path = config_path();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return UiPrefs::default(),
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Failed to read config; using defaults");
            return UiPrefs::default();
        }
    };
    match toml::from_str::<UiPrefs>(&text) {
        Ok(prefs) => prefs.sanitized(),
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Failed to parse config; using defaults");
            UiPrefs::default()
        }
    }
}

/// Save preferences atomically. Errors are logged, never fatal: losing a preference write
/// must not take the app down.
pub fn save_prefs(prefs: &UiPrefs) {
    let path = config_path();
    let text = match toml::to_string_pretty(prefs) {
        Ok(text) => text,
        Err(e) => {
            warn!(error = %e, "Failed to serialize config");
            return;
        }
    };
    if let Err(e) = write_atomic(&path, text.as_bytes()) {
        warn!(path = %path.display(), error = %e, "Failed to write config");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_survive_empty_and_partial_toml() {
        let empty: UiPrefs = match toml::from_str("") {
            Ok(p) => p,
            Err(e) => panic!("empty config must parse: {e}"),
        };
        assert_eq!(empty, UiPrefs::default());

        let partial: UiPrefs = match toml::from_str("theme = \"light\"") {
            Ok(p) => p,
            Err(e) => panic!("partial config must parse: {e}"),
        };
        assert_eq!(partial.theme, ThemeChoice::Light);
        assert_eq!(partial.window, WindowPrefs::default());
    }

    #[test]
    fn round_trips_through_toml() {
        let prefs = UiPrefs {
            theme: ThemeChoice::Light,
            window: WindowPrefs {
                width: 1600.0,
                height: 900.0,
                maximized: true,
            },
            layout: LayoutPrefs {
                sidebar_px: 250.0,
                editor_px: 400.0,
            },
        };
        let text = match toml::to_string_pretty(&prefs) {
            Ok(t) => t,
            Err(e) => panic!("serialize: {e}"),
        };
        let back: UiPrefs = match toml::from_str(&text) {
            Ok(p) => p,
            Err(e) => panic!("reparse: {e}"),
        };
        assert_eq!(back, prefs);
    }

    #[test]
    fn sanitize_clamps_nonsense() {
        let p = UiPrefs {
            window: WindowPrefs {
                width: f32::NAN,
                height: 50.0,
                maximized: false,
            },
            layout: LayoutPrefs {
                sidebar_px: 10_000.0,
                editor_px: -3.0,
            },
            ..UiPrefs::default()
        }
        .sanitized();
        assert_eq!(p.window.width, 1280.0);
        assert_eq!(p.window.height, 400.0);
        assert_eq!(p.layout.sidebar_px, 560.0);
        assert_eq!(p.layout.editor_px, 220.0);
    }
}
