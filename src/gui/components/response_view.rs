use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::models::ResponseModel;

/// Response view component:
/// - shows response stats (status, duration with `{:?}`, body size)
/// - optional headers section (toggle)
/// - optional pretty JSON toggle (best-effort formatting handled by parent)
/// - body display (scrollable)
///
/// This component is intentionally "thin": it does not own state. It renders from
/// the provided flags/data and emits `Message` events.
#[derive(Debug, Clone)]
pub struct ResponseView<'a> {
    pub response: Option<&'a ResponseModel>,
    pub error: Option<&'a str>,
    pub show_headers: bool,
    pub pretty_json: bool,

    /// Pre-formatted body text (e.g. pretty JSON) supplied by parent.
    pub body_text: Option<&'a str>,

    /// Sizing
    pub headers_height: Length,
}

impl<'a> ResponseView<'a> {
    pub fn new() -> Self {
        Self {
            response: None,
            error: None,
            show_headers: true,
            pretty_json: true,
            body_text: None,
            headers_height: Length::Fixed(180.0),
        }
    }

    pub fn response(mut self, response: Option<&'a ResponseModel>) -> Self {
        self.response = response;
        self
    }

    pub fn error(mut self, error: Option<&'a str>) -> Self {
        self.error = error;
        self
    }

    pub fn show_headers(mut self, show: bool) -> Self {
        self.show_headers = show;
        self
    }

    pub fn pretty_json(mut self, pretty: bool) -> Self {
        self.pretty_json = pretty;
        self
    }

    pub fn body_text(mut self, body: Option<&'a str>) -> Self {
        self.body_text = body;
        self
    }

    pub fn headers_height(mut self, h: Length) -> Self {
        self.headers_height = h;
        self
    }

    pub fn view(self) -> Element<'a, Message> {
        let toggles = self.view_toggles();

        let body = if let Some(err) = self.error {
            container(
                column![
                    text("Error").size(18),
                    Space::new().height(Length::Fixed(6.0)),
                    text(err).size(14),
                ]
                .spacing(6),
            )
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else if let Some(resp) = self.response {
            self.view_response(resp)
        } else {
            container(
                column![
                    text("Ready").size(18),
                    text("Send a request to see the response.").size(14),
                ]
                .spacing(6),
            )
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        };

        let content = column![
            row![text("Response").size(20)].align_y(Vertical::Center),
            toggles,
            body
        ]
        .spacing(10)
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_toggles(&self) -> Element<'a, Message> {
        let pretty_btn = button(text(if self.pretty_json {
            "Pretty JSON: on"
        } else {
            "Pretty JSON: off"
        }))
        .padding(10)
        .on_press(Message::TogglePrettyJson);

        let headers_btn = button(text(if self.show_headers {
            "Show headers: on"
        } else {
            "Show headers: off"
        }))
        .padding(10)
        .on_press(Message::ToggleShowHeaders);

        row![pretty_btn, headers_btn]
            .spacing(10)
            .align_y(Vertical::Center)
            .into()
    }

    fn view_response(&self, resp: &'a ResponseModel) -> Element<'a, Message> {
        let stats = format!(
            "Status: {} {} • Duration: {:?} • Body: {} bytes",
            resp.status.code,
            resp.status.reason,
            resp.duration,
            resp.body.len()
        );

        let mut col = column![text(stats).size(14)].spacing(10);

        if self.show_headers {
            let mut headers_lines = String::new();
            for (name, value) in resp.headers.iter() {
                let value_str = value.to_str().unwrap_or("<non-utf8>");
                headers_lines.push_str(name.as_str());
                headers_lines.push_str(": ");
                headers_lines.push_str(value_str);
                headers_lines.push('\n');
            }

            let headers_block = if headers_lines.is_empty() {
                "<no headers>".to_string()
            } else {
                headers_lines
            };

            col = col.push(text("Headers").size(16)).push(
                scrollable(text(headers_block).size(12))
                    .height(self.headers_height)
                    .width(Length::Fill),
            );
        }

        let body_text = self.body_text.map(|s| s.to_string()).unwrap_or_else(|| {
            if resp.body.is_empty() {
                "<empty body>".to_string()
            } else {
                resp.body.clone()
            }
        });

        col = col.push(text("Body").size(16)).push(
            scrollable(text(body_text).size(12))
                .height(Length::Fill)
                .width(Length::Fill),
        );

        container(col)
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
