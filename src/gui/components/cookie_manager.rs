//! Cookie manager: lists the session jar's cookies with per-cookie delete, an add row,
//! and a clear-all action.

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::messages::CookieDraftField;
use crate::gui::state::CookieDraft;
use crate::gui::theme;
use crate::http::CookieView;

/// Render the cookie manager from a snapshot of the jar plus the add-row buffers.
pub fn view(cookies: &[CookieView], draft: &CookieDraft) -> Element<'static, Message> {
    let header = row![
        text(format!("Cookies ({})", cookies.len())).size(18),
        Space::new().width(Length::Fill),
        button(text("Clear all").size(13))
            .padding(8)
            .style(theme::flat)
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
        let scope = if c.host_only {
            format!("    domain {} (host-only) · path {}", c.domain, c.path)
        } else {
            format!("    domain {} · path {}", c.domain, c.path)
        };
        let delete = button(theme::icons::icon(theme::icons::TRASH, 11.0).style(theme::muted))
            .padding(4)
            .style(theme::flat)
            .on_press(Message::CookieDelete {
                domain: c.domain.clone(),
                path: c.path.clone(),
                name: c.name.clone(),
            });
        list = list.push(
            row![
                column![
                    text(format!("{} = {}", c.name, c.value))
                        .size(13)
                        .font(theme::fonts::MONO),
                    text(scope).size(11).style(theme::muted),
                ]
                .spacing(2)
                .width(Length::Fill),
                delete,
            ]
            .spacing(6)
            .align_y(Vertical::Center),
        );
    }

    let field = |placeholder: &'static str, value: &str, kind: CookieDraftField| {
        text_input(placeholder, value)
            .on_input(move |s| Message::CookieDraftChanged(kind, s))
            .padding(6)
            .size(12)
    };
    let add_row = row![
        field("domain", &draft.domain, CookieDraftField::Domain).width(Length::FillPortion(3)),
        field("/path", &draft.path, CookieDraftField::Path).width(Length::FillPortion(2)),
        field("name", &draft.name, CookieDraftField::Name).width(Length::FillPortion(2)),
        field("value", &draft.value, CookieDraftField::Value).width(Length::FillPortion(3)),
        button(text("Add").size(12))
            .padding(6)
            .on_press(Message::CookieAdd),
    ]
    .spacing(6)
    .align_y(Vertical::Center);

    container(
        column![
            header,
            scrollable(list).height(Length::Fill).width(Length::Fill),
            add_row
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
