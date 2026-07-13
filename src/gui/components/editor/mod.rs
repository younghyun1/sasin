//! Request editor for the active tab: method/url/send + Params/Headers/Auth/Body/Settings panels.

mod panels;

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::components::tab_strip;
use crate::gui::messages::EditorPanel;
use crate::gui::state::Tab;
use crate::gui::theme;
use crate::model::HttpRequest;
use crate::models::HttpMethod;

/// Render the editor for an HTTP request node + its tab buffers.
pub fn view<'a>(
    req: &'a HttpRequest,
    tab: &'a Tab,
    hl: iced::highlighter::Theme,
    snippet_lang: crate::interop::SnippetLang,
) -> Element<'a, Message> {
    let method = pick_list(
        HttpMethod::all(),
        HttpMethod::parse(&req.method),
        Message::MethodChanged,
    )
    .padding(10);
    let url = text_input("https://example.com", &req.url)
        .on_input(Message::UrlChanged)
        .padding(10)
        .size(15)
        .width(Length::Fill);
    let send_label = if tab.sending { "Sending…" } else { "Send" };
    let send = button(text(send_label).size(15))
        .padding([10, 24])
        .style(theme::selected)
        .on_press_maybe((!tab.sending).then_some(Message::SendPressed));
    let top = row![method, url, send]
        .spacing(10)
        .align_y(Vertical::Center);

    let actions = row![
        text(&tab.name).size(14).font(theme::fonts::UI_SEMIBOLD),
        Space::new().width(Length::Fill),
        pick_list(
            crate::interop::SnippetLang::all(),
            Some(snippet_lang),
            Message::SnippetLangChanged
        )
        .padding(6)
        .text_size(12),
        button(text("Copy").size(13))
            .padding(8)
            .style(theme::flat)
            .on_press(Message::CopySnippet),
        button(text("Save").size(13))
            .padding(8)
            .style(theme::flat)
            .on_press(Message::SaveActiveTab),
        button(text("Cancel").size(13))
            .padding(8)
            .style(theme::flat)
            .on_press_maybe(tab.sending.then_some(Message::CancelPressed)),
    ]
    .spacing(10)
    .align_y(Vertical::Center);

    let content = column![
        top,
        Space::new().height(Length::Fixed(8.0)),
        actions,
        Space::new().height(Length::Fixed(8.0)),
        panel_bar(tab.panel),
        panels::view(req, tab, hl),
    ]
    .spacing(8)
    .width(Length::Fill);

    container(content)
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn panel_bar(active: EditorPanel) -> Element<'static, Message> {
    let make = |label: &'static str, panel: EditorPanel| {
        tab_strip::tab(
            text(label).size(13),
            panel == active,
            Message::SelectPanel(panel),
        )
    };
    row![
        make("Params", EditorPanel::Params),
        make("Headers", EditorPanel::Headers),
        make("Auth", EditorPanel::Auth),
        make("Body", EditorPanel::Body),
        make("Scripts", EditorPanel::Scripts),
        make("Docs", EditorPanel::Docs),
        make("Settings", EditorPanel::Settings),
    ]
    .spacing(2)
    .into()
}
