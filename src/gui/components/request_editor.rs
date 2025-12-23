/// Request editor component: method/url/headers/body inputs + action buttons.
///
/// This is designed to be *thin* and reusable:
/// - It does not own state; it renders from the provided values.
/// - It emits `Message` events to update the parent `App`.
///
/// UI goals:
/// - Method selection: dropdown (`pick_list`)
/// - URL: single-line input
/// - Headers: multi-line editor box
/// - Body: multi-line editor box
/// - Actions: Send/Cancel + Save Template/Save Dataset + history/template actions
///
/// Notes:
/// - `text_editor` has internal scrolling; we size it with fixed pixel heights.
/// - Keep this file focused on the request-side UI only.
///
/// IMPORTANT:
/// Iced 0.14 `text_editor::Content` is generic over a text renderer, which makes it awkward to
/// construct ad-hoc editor content in a component without pulling the renderer type through.
/// For now, the headers editor uses a multiline `text_input` (lean + functional).
use iced::alignment::Vertical;
use iced::widget::{
    Space, button, column, container, pick_list, row, text, text_editor, text_input,
};
use iced::{Element, Length};

use crate::gui::Message;
use crate::models::HttpMethod;

#[derive(Debug, Clone)]
pub struct RequestEditor<'a> {
    // Request fields
    pub method: HttpMethod,
    pub url: &'a str,

    // Multi-line editors (owned by the parent App)
    pub headers_text: &'a str,
    pub body_content: &'a text_editor::Content,

    // Dataset/template context
    pub request_name: &'a str,
    pub autosave_enabled: bool,
    pub dataset_dirty: bool,

    // Async state
    pub sending: bool,

    // Sizing
    pub body_height_px: f32,
    pub headers_height_px: f32,
}

impl<'a> RequestEditor<'a> {
    pub fn new(
        method: HttpMethod,
        url: &'a str,
        headers_text: &'a str,
        body_content: &'a text_editor::Content,
        request_name: &'a str,
    ) -> Self {
        Self {
            method,
            url,
            headers_text,
            body_content,
            request_name,
            autosave_enabled: true,
            dataset_dirty: false,
            sending: false,
            body_height_px: 260.0,
            headers_height_px: 90.0,
        }
    }

    pub fn autosave_enabled(mut self, enabled: bool) -> Self {
        self.autosave_enabled = enabled;
        self
    }

    pub fn dataset_dirty(mut self, dirty: bool) -> Self {
        self.dataset_dirty = dirty;
        self
    }

    pub fn sending(mut self, sending: bool) -> Self {
        self.sending = sending;
        self
    }

    pub fn body_height_px(mut self, px: f32) -> Self {
        self.body_height_px = px;
        self
    }

    pub fn headers_height_px(mut self, px: f32) -> Self {
        self.headers_height_px = px;
        self
    }

    pub fn view(self) -> Element<'a, Message> {
        let top_bar = self.view_top_bar();

        let dataset_row = self.view_dataset_row();

        // Headers editor:
        // Use multiline `text_input` for now to avoid `text_editor::Content<_>` renderer generic issues.
        let headers_editor = text_input(
            "Header: Value (one per line). Example: Accept: application/json",
            self.headers_text,
        )
        .on_input(Message::HeadersChanged)
        .padding(12)
        .size(14)
        .width(Length::Fill);

        let body_editor = text_editor(self.body_content)
            .placeholder("Request body (raw text)…")
            .on_action(|action| {
                let mut content = self.body_content.clone();
                content.perform(action);
                Message::BodyChanged(content.text())
            })
            .height(self.body_height_px);

        let request_name_input = text_input("Request name…", self.request_name)
            // Parent can decide what this means; immediate-mutation mode may
            // interpret as "rename selected request".
            .on_input(|s| Message::RenameRequestPressed(0, s))
            .padding(12)
            .size(14)
            .width(Length::Fill);

        let actions = self.view_actions_row();

        let content = column![
            top_bar,
            Space::new().height(Length::Fixed(10.0)),
            dataset_row,
            Space::new().height(Length::Fixed(10.0)),
            text("Request Name").size(16),
            request_name_input,
            Space::new().height(Length::Fixed(10.0)),
            text("Headers").size(16),
            headers_editor,
            Space::new().height(Length::Fixed(10.0)),
            text("Body").size(16),
            body_editor,
            Space::new().height(Length::Fixed(10.0)),
            actions,
        ]
        .spacing(10)
        .width(Length::Fill);

        container(content)
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_top_bar(&self) -> Element<'a, Message> {
        let method_dropdown =
            pick_list(HttpMethod::all(), Some(self.method), Message::MethodChanged).padding(10);

        let url_input = text_input("https://example.com", self.url)
            .on_input(Message::UrlChanged)
            .padding(12)
            .size(16)
            .width(Length::Fill);

        let send_label = if self.sending { "Sending…" } else { "Send" };
        let send_btn = button(text(send_label).size(16))
            .padding(12)
            .on_press_maybe((!self.sending).then_some(Message::SendPressed));

        row![method_dropdown, url_input, send_btn]
            .spacing(12)
            .align_y(Vertical::Center)
            .into()
    }

    fn view_dataset_row(&self) -> Element<'a, Message> {
        let autosave_label = if self.autosave_enabled {
            "Autosave: On"
        } else {
            "Autosave: Off"
        };

        let dirty_label = if self.dataset_dirty { "• dirty" } else { "" };

        row![
            text(format!("Dataset {dirty_label}")).size(16),
            Space::new().width(Length::Fill),
            button(text("Open…").size(14))
                .padding(10)
                .on_press(Message::OpenDatasetPressed),
            button(text("Save As…").size(14))
                .padding(10)
                .on_press(Message::SaveDatasetAsPressed),
            button(text(autosave_label).size(14))
                .padding(10)
                .on_press(Message::ToggleAutosave),
        ]
        .spacing(10)
        .align_y(Vertical::Center)
        .into()
    }

    fn view_actions_row(&self) -> Element<'a, Message> {
        let cancel_btn = button(text("Cancel").size(14))
            .padding(10)
            .on_press_maybe(self.sending.then_some(Message::CancelPressed));

        let clear_btn = button(text("Clear Output").size(14))
            .padding(10)
            .on_press(Message::ClearPressed);

        let save_request_btn = button(text("Save Request").size(14))
            .padding(10)
            .on_press(Message::SaveRequestPressed);

        let save_dataset_btn = button(text("Save Dataset").size(14))
            .padding(10)
            .on_press(Message::SaveDataset);

        row![cancel_btn, clear_btn, save_request_btn, save_dataset_btn]
            .spacing(10)
            .align_y(Vertical::Center)
            .into()
    }
}
