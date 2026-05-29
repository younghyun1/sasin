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
pub mod runtime;
pub mod scripting;
pub mod setup_logger;
pub mod storage;
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

    // Iced 0.14 builder: boot (loads the workspace) + update + view, with a dynamic title.
    iced::application(
        crate::gui::App::new,
        crate::gui::App::update,
        crate::gui::App::view,
    )
    .title(crate::gui::App::title)
    .subscription(crate::gui::App::subscription)
    .run()
}
