//! Cookie manager: lists the session jar's cookies and offers a clear-all action.

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::theme;
use crate::http::CookieView;

/// Render the cookie manager from a snapshot of the jar.
pub fn view(cookies: &[CookieView]) -> Element<'static, Message> {
    let header = row![
        text(format!("Cookies ({})", cookies.len())).size(18),
        Space::new().width(Length::Fill),
        button(text("Clear all").size(13))
            .padding(8)
            .on_press_maybe((!cookies.is_empty()).then_some(Message::ClearCookies)),
        button(text("Close").size(13))
            .padding(8)
            .on_press(Message::ToggleCookieManager),
    ]
    .spacing(10)
    .align_y(Vertical::Center);

    let mut list = column![].spacing(6).width(Length::Fill);
    if cookies.is_empty() {
        list = list
            .push(text("No cookies stored. Send a request whose response sets cookies.").size(13));
    }
    for c in cookies {
        list = list.push(
            column![
                text(format!("{} = {}", c.name, c.value))
                    .size(13)
                    .font(theme::fonts::MONO),
                text(format!("    domain {} · path {}", c.domain, c.path)).size(11),
            ]
            .spacing(2),
        );
    }

    container(
        column![
            header,
            scrollable(list).height(Length::Fill).width(Length::Fill)
        ]
        .spacing(10)
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .style(theme::panel)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
