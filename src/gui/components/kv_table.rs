//! Reusable key/value table (params, headers, url-encoded fields) with per-row enable + delete.

use iced::widget::{button, checkbox, column, row, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::messages::{KvOp, KvTarget};
use crate::model::KvEntry;

/// Render an editable key/value table targeting `target`. Borrows `entries` from the node.
pub fn view<'a>(
    target: KvTarget,
    entries: &'a [KvEntry],
    key_placeholder: &'a str,
    value_placeholder: &'a str,
) -> Element<'a, Message> {
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let enabled =
            checkbox(entry.enabled).on_toggle(move |b| Message::Kv(target, KvOp::Toggle(i, b)));
        let key = text_input(key_placeholder, &entry.key)
            .on_input(move |s| Message::Kv(target, KvOp::Key(i, s)))
            .padding(6)
            .size(13)
            .width(Length::FillPortion(2));
        let value = text_input(value_placeholder, &entry.value)
            .on_input(move |s| Message::Kv(target, KvOp::Value(i, s)))
            .padding(6)
            .size(13)
            .width(Length::FillPortion(3));
        let delete = button(text("✕").size(12))
            .padding(6)
            .on_press(Message::Kv(target, KvOp::Remove(i)));
        rows.push(row![enabled, key, value, delete].spacing(6).into());
    }
    rows.push(
        button(text("+ Add").size(12))
            .padding(6)
            .on_press(Message::Kv(target, KvOp::Add))
            .into(),
    );
    column(rows).spacing(6).width(Length::Fill).into()
}
