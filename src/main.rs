#![windows_subsystem = "windows"]

use crate::setup_logger::setup_logger;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod build_info;
pub mod gui;
pub mod http;
pub mod interop;
pub mod model;
pub mod models;
pub mod persist;
pub mod runner;
pub mod runtime;
pub mod scripting;
pub mod setup_logger;
pub mod storage;
pub mod watch;
pub mod ws;

fn main() -> iced::Result {
    // These guards need to stay alive for the global logger to work.
    // We can't `.await` directly in `main` since we're using Iced's `run` entrypoint,
    // so we block on a tiny runtime just for logger initialization.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .expect("failed to build tokio runtime");

    let _guards = rt.block_on(async { setup_logger().await });

    // UI preferences drive the initial window and theme; the app tracks and re-saves them.
    let prefs = crate::persist::load_prefs();
    let window = iced::window::Settings {
        size: prefs.window_size(),
        min_size: Some(iced::Size::new(960.0, 600.0)),
        maximized: prefs.window.maximized,
        ..iced::window::Settings::default()
    };

    // Iced 0.14 builder: boot (loads the workspace) + update + view, with a dynamic title.
    iced::application(
        move || crate::gui::App::new(prefs),
        crate::gui::App::update,
        crate::gui::App::view,
    )
    .title(crate::gui::App::title)
    .theme(crate::gui::App::theme)
    .subscription(crate::gui::App::subscription)
    .window(window)
    // Close is intercepted so preferences flush before exit (see `App::close_requested`).
    .exit_on_close_request(false)
    .run()
}
