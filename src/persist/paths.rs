use std::env;
use std::path::PathBuf;

/// Returns the base directory for app state on the current platform.
///
/// - Windows: `%APPDATA%/sasin`
/// - macOS: `~/Library/Application Support/sasin`
/// - Linux/Unix: `$XDG_CONFIG_HOME/sasin` or `~/.config/sasin`
pub fn app_state_dir() -> PathBuf {
    // Touch `home_dir` on all builds so the helper is not considered dead code.
    // (Some platforms/configurations may compile out the branches that call it.)
    let _ = home_dir();

    // Windows
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = env::var("APPDATA") {
            if !appdata.trim().is_empty() {
                return PathBuf::from(appdata).join("sasin");
            }
        }

        // Fallback: use user profile if APPDATA isn't set.
        if let Ok(userprofile) = env::var("USERPROFILE") {
            if !userprofile.trim().is_empty() {
                return PathBuf::from(userprofile)
                    .join("AppData")
                    .join("Roaming")
                    .join("sasin");
            }
        }
    }

    // macOS
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home_dir() {
            return home
                .join("Library")
                .join("Application Support")
                .join("sasin");
        }
    }

    // Linux/Unix (including *BSD)
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = env::var("XDG_CONFIG_HOME")
            && !xdg.trim().is_empty()
        {
            return PathBuf::from(xdg).join("sasin");
        }

        if let Some(home) = home_dir() {
            return home.join(".config").join("sasin");
        }
    }

    // Last-resort fallback
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".sasin")
}

/// Default path for the app config file.
///
/// This stores small app state like "last opened dataset path".
pub fn default_config_path() -> PathBuf {
    app_state_dir().join("config.sasin")
}

/// Default path for the dataset file.
///
/// This stores saved request templates (method/url/headers/body).
pub fn default_dataset_path() -> PathBuf {
    app_state_dir().join("dataset.sasin")
}

/// Minimal home dir helper without extra dependencies.
///
/// Uses:
/// - Unix/macOS: `$HOME`
/// - Windows: `%USERPROFILE%` (or `%HOMEDRIVE%%HOMEPATH%` fallback)
fn home_dir() -> Option<PathBuf> {
    // Unix/macOS
    if let Ok(home) = env::var("HOME")
        && !home.trim().is_empty()
    {
        return Some(PathBuf::from(home));
    }

    // Windows
    if let Ok(userprofile) = env::var("USERPROFILE")
        && !userprofile.trim().is_empty()
    {
        return Some(PathBuf::from(userprofile));
    }

    let homedrive = env::var("HOMEDRIVE").ok();
    let homepath = env::var("HOMEPATH").ok();
    match (homedrive, homepath) {
        (Some(d), Some(p)) if !d.trim().is_empty() && !p.trim().is_empty() => {
            Some(PathBuf::from(format!("{d}{p}")))
        }
        _ => None,
    }
}
