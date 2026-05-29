//! Request editor for the active tab: method/url/headers/body + send/save actions.
//!
//! P2 keeps headers as raw `Name: Value` text and body as raw text; the structured
//! params/headers/auth/body-mode panels arrive in P3.

use iced::alignment::Vertical;
use iced::widget::{
    Space, button, column, container, pick_list, row, text, text_editor, text_input,
};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::state::{Tab, TabKind};
use crate::models::HttpMethod;

/// Render the request editor for `tab`. Borrows the tab (the body editor needs `&Content`).
pub fn view(tab: &Tab) -> Element<'_, Message> {
    let method = pick_list(HttpMethod::all(), Some(tab.method), Message::MethodChanged).padding(10);

    let url = text_input("https://example.com", &tab.url)
        .on_input(Message::UrlChanged)
        .padding(10)
        .size(15)
        .width(Length::Fill);

    let send_label = if tab.sending { "Sending…" } else { "Send" };
    let send = button(text(send_label).size(15))
        .padding(10)
        .on_press_maybe((!tab.sending).then_some(Message::SendPressed));

    let top = row![method, url, send]
        .spacing(10)
        .align_y(Vertical::Center);

    let actions = row![
        text(&tab.name).size(14),
        Space::new().width(Length::Fill),
        button(text("Save").size(13))
            .padding(8)
            .on_press(Message::SaveActiveTab),
        button(text("Cancel").size(13))
            .padding(8)
            .on_press_maybe(tab.sending.then_some(Message::CancelPressed)),
    ]
    .spacing(10)
    .align_y(Vertical::Center);

    let headers = text_input(
        "Header: Value (one per line). Example: Accept: application/json",
        &tab.headers_text,
    )
    .on_input(Message::HeadersChanged)
    .padding(10)
    .size(13)
    .width(Length::Fill);

    let body = text_editor(&tab.body)
        .placeholder("Request body (raw text)…")
        .on_action(Message::BodyAction)
        .height(Length::Fixed(220.0));

    let ws_note: Element<'_, Message> = if tab.kind == TabKind::Ws {
        text("WebSocket sessions are interactive — console arrives in P7.")
            .size(12)
            .into()
    } else {
        Space::new().height(Length::Fixed(0.0)).into()
    };

    let content = column![
        top,
        Space::new().height(Length::Fixed(8.0)),
        actions,
        Space::new().height(Length::Fixed(8.0)),
        text("Headers").size(15),
        headers,
        Space::new().height(Length::Fixed(8.0)),
        text("Body").size(15),
        body,
        ws_note,
    ]
    .spacing(8)
    .width(Length::Fill);

    container(content)
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
