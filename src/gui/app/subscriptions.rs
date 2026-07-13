//! All live subscriptions: websocket sessions, the workspace file watch, window events,
//! and the debounced config-flush tick.

use std::time::Duration;

use iced::Subscription;

use crate::gui::Message;
use crate::gui::app::App;
use crate::ws;

impl App {
    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs: Vec<Subscription<Message>> = self
            .ws
            .iter()
            .filter(|rt| rt.active)
            .map(|rt| {
                ws::connect(rt.path.clone(), rt.config.clone())
                    .map(|(path, event)| Message::Ws(path, event))
            })
            .collect();
        subs.push(crate::watch::watch(&self.workspace_dir).map(|()| Message::WorkspaceChanged));
        subs.push(iced::window::resize_events().map(|(_id, size)| Message::WindowResized(size)));
        subs.push(iced::window::close_requests().map(Message::WindowCloseRequested));
        // Debounce preference writes: only tick while something actually changed.
        if self.config_dirty {
            subs.push(iced::time::every(Duration::from_secs(2)).map(|_| Message::ConfigFlushTick));
        }
        Subscription::batch(subs)
    }
}
