//! Global keyboard shortcuts, delivered via `event::listen_with` so chords work even while a
//! text widget has focus (`keyboard::on_key_press` only fires for uncaptured events).

use iced::Subscription;
use iced::event::{self, Event};
use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyEvent, Key, Modifiers};

use crate::gui::Message;
use crate::gui::messages::TreeMsg;

pub fn subscription() -> Subscription<Message> {
    event::listen_with(|event, _status, _window| match event {
        Event::Keyboard(KeyEvent::KeyPressed { key, modifiers, .. }) => {
            map_key(key.as_ref(), modifiers)
        }
        _ => None,
    })
}

/// Pure chord → message mapping. `modifiers.command()` is Ctrl on Linux/Windows, Cmd on macOS.
fn map_key(key: Key<&str>, modifiers: Modifiers) -> Option<Message> {
    if modifiers.command() {
        return match key {
            Key::Named(Named::Enter) => Some(Message::SendPressed),
            Key::Character("s") => Some(Message::SaveActiveTab),
            Key::Character("w") => Some(Message::CloseActiveTab),
            Key::Character("t") => Some(Message::NewRequest),
            Key::Character("f") => Some(Message::FocusResponseSearch),
            _ => None,
        };
    }
    // Escape cancels an in-flight tree rename (a no-op when none is active).
    if matches!(key, Key::Named(Named::Escape)) {
        return Some(Message::Tree(TreeMsg::RenameCancel));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_chords_map() {
        let m = Modifiers::CTRL;
        assert!(matches!(
            map_key(Key::Named(Named::Enter), m),
            Some(Message::SendPressed)
        ));
        assert!(matches!(
            map_key(Key::Character("s"), m),
            Some(Message::SaveActiveTab)
        ));
        assert!(matches!(
            map_key(Key::Character("w"), m),
            Some(Message::CloseActiveTab)
        ));
        assert!(matches!(
            map_key(Key::Character("t"), m),
            Some(Message::NewRequest)
        ));
        assert!(matches!(
            map_key(Key::Character("f"), m),
            Some(Message::FocusResponseSearch)
        ));
        assert!(map_key(Key::Character("q"), m).is_none());
    }

    #[test]
    fn plain_keys_do_not_fire_chords() {
        let none = Modifiers::empty();
        assert!(map_key(Key::Character("s"), none).is_none());
        assert!(matches!(
            map_key(Key::Named(Named::Escape), none),
            Some(Message::Tree(TreeMsg::RenameCancel))
        ));
    }
}
