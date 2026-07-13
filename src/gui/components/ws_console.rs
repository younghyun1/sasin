//! WebSocket console: connect bar, transcript, saved-message buttons, and a message composer.

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::state::{WsDir, WsRuntime};
use crate::gui::theme;
use crate::model::{WsKind, WsRequest};

/// Render the console for a websocket node. `rt` is the live session when this node is connected.
pub fn view<'a>(req: &'a WsRequest, rt: Option<&'a WsRuntime>) -> Element<'a, Message> {
    let active = rt.is_some_and(|r| r.active);
    let connected = rt.is_some_and(|r| r.connected);

    let connect = if active {
        button(text("Disconnect").size(14))
            .padding(10)
            .on_press(Message::WsDisconnect)
    } else {
        button(text("Connect").size(14))
            .padding(10)
            .on_press(Message::WsConnect)
    };
    let url = if req.url.is_empty() {
        "wss://…"
    } else {
        req.url.as_str()
    };
    let state_pill: Element<'a, Message> = if connected {
        pill_text("connected", true)
    } else {
        pill_text("disconnected", false)
    };
    let bar = row![
        text(format!("WebSocket — {url}")).size(14),
        state_pill,
        Space::new().width(Length::Fill),
        connect,
    ]
    .spacing(10)
    .align_y(Vertical::Center);

    let transcript: Element<'a, Message> = match rt {
        Some(r) if !r.transcript.is_empty() => {
            let mut col = column![].spacing(2).width(Length::Fill);
            for line in &r.transcript {
                let glyph = match line.dir {
                    WsDir::In => theme::icons::ARROW_LEFT,
                    WsDir::Out => theme::icons::ARROW_RIGHT,
                    WsDir::Info => theme::icons::DOT,
                };
                col = col.push(
                    row![
                        theme::icons::icon(glyph, 11.0).style(theme::muted),
                        text(line.text.clone()).size(12).font(theme::fonts::MONO),
                    ]
                    .spacing(6)
                    .align_y(Vertical::Center),
                );
            }
            scrollable(col)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        }
        _ => container_note("Not connected. Click Connect to open the session."),
    };

    let mut saved = row![].spacing(6);
    for (i, message) in req.messages.iter().enumerate() {
        let label = if message.name.is_empty() {
            format!("msg {i}")
        } else {
            message.name.clone()
        };
        saved = saved.push(
            button(text(label).size(12))
                .padding(6)
                .on_press(Message::WsSendSaved(i)),
        );
    }

    let composer_text = rt.map(|r| r.composer.as_str()).unwrap_or("");
    let kind = rt.map(|r| r.kind).unwrap_or_default();
    let composer = row![
        pick_list(WsKind::all(), Some(kind), Message::WsKindChanged).padding(8),
        text_input("message…", composer_text)
            .on_input(Message::WsComposerChanged)
            .padding(8)
            .size(13)
            .width(Length::Fill),
        button(text("Send").size(13))
            .padding(8)
            .on_press_maybe(connected.then_some(Message::WsSend)),
    ]
    .spacing(8)
    .align_y(Vertical::Center);

    column![bar, transcript, saved, composer]
        .spacing(8)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn container_note(msg: &str) -> Element<'_, Message> {
    text(msg).size(13).into()
}

/// Small connected/disconnected state label colored by success/danger.
fn pill_text(label: &'static str, ok: bool) -> Element<'static, Message> {
    text(label)
        .size(11)
        .style(move |t: &iced::Theme| {
            let p = t.extended_palette();
            iced::widget::text::Style {
                color: Some(if ok {
                    p.success.base.color
                } else {
                    p.danger.base.color
                }),
            }
        })
        .into()
}
