//! Response panel: stats line, a tab bar (Body / Headers / Cookies / Preview), a body search box,
//! and a save-as-example action. The component is stateless — it renders from the flags the parent
//! ([`App`](crate::gui::app::App)) owns and emits messages back.

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::messages::ResponseTab;
use crate::gui::theme;
use crate::models::ResponseModel;

/// Thin, stateless response renderer.
#[derive(Debug, Clone)]
pub struct ResponseView<'a> {
    response: Option<&'a ResponseModel>,
    error: Option<&'a str>,
    tab: ResponseTab,
    pretty_json: bool,
    search: &'a str,
}

impl<'a> ResponseView<'a> {
    pub fn new() -> Self {
        Self {
            response: None,
            error: None,
            tab: ResponseTab::Body,
            pretty_json: true,
            search: "",
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

    pub fn tab(mut self, tab: ResponseTab) -> Self {
        self.tab = tab;
        self
    }

    pub fn pretty_json(mut self, pretty: bool) -> Self {
        self.pretty_json = pretty;
        self
    }

    pub fn search(mut self, search: &'a str) -> Self {
        self.search = search;
        self
    }

    pub fn view(self) -> Element<'a, Message> {
        let body: Element<'a, Message> = if let Some(err) = self.error {
            note("Error", err)
        } else if let Some(resp) = self.response {
            self.view_response(resp)
        } else {
            note("Ready", "Send a request to see the response.")
        };

        container(
            column![
                row![text("Response").size(20)].align_y(Vertical::Center),
                body
            ]
            .spacing(10)
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_response(&self, resp: &'a ResponseModel) -> Element<'a, Message> {
        let stats = text(format!(
            "Status: {} {} • Duration: {:?} • Body: {} bytes",
            resp.status.code,
            resp.status.reason,
            resp.duration,
            resp.body.len()
        ))
        .size(14);

        let content = match self.tab {
            ResponseTab::Body => self.body_view(resp),
            ResponseTab::Headers => headers_view(resp),
            ResponseTab::Cookies => cookies_view(resp),
            ResponseTab::Preview => preview_view(resp),
        };

        column![stats, self.toolbar(), content]
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn toolbar(&self) -> Element<'a, Message> {
        let tab_btn = |label: &'static str, tab: ResponseTab| {
            let b = button(text(label).size(12))
                .padding(6)
                .on_press(Message::SelectResponseTab(tab));
            if tab == self.tab {
                b.style(theme::selected)
            } else {
                b.style(theme::flat)
            }
        };
        let mut bar = row![
            tab_btn("Body", ResponseTab::Body),
            tab_btn("Headers", ResponseTab::Headers),
            tab_btn("Cookies", ResponseTab::Cookies),
            tab_btn("Preview", ResponseTab::Preview),
        ]
        .spacing(4)
        .align_y(Vertical::Center);

        if self.tab == ResponseTab::Body {
            bar = bar.push(
                button(
                    text(if self.pretty_json {
                        "Pretty: on"
                    } else {
                        "Pretty: off"
                    })
                    .size(12),
                )
                .padding(6)
                .style(theme::flat)
                .on_press(Message::TogglePrettyJson),
            );
            bar = bar.push(
                text_input("search body…", self.search)
                    .on_input(Message::ResponseSearchChanged)
                    .padding(6)
                    .size(12)
                    .width(Length::Fixed(180.0)),
            );
        }
        bar = bar.push(Space::new().width(Length::Fill));
        bar = bar.push(
            button(text("Save example").size(12))
                .padding(6)
                .style(theme::flat)
                .on_press(Message::SaveAsExample),
        );
        bar.into()
    }

    fn body_view(&self, resp: &'a ResponseModel) -> Element<'a, Message> {
        let formatted = format_body(&resp.body, self.pretty_json);
        let (shown, header) = filter_search(&formatted, self.search);
        let mut col = column![].spacing(4).width(Length::Fill);
        if let Some(h) = header {
            col = col.push(text(h).size(11));
        }
        col = col.push(text(shown).size(12));
        scrollable(col)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

impl<'a> Default for ResponseView<'a> {
    fn default() -> Self {
        Self::new()
    }
}

fn note<'a>(title: &'a str, body: &'a str) -> Element<'a, Message> {
    container(column![text(title).size(18), text(body).size(14)].spacing(6))
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn headers_view(resp: &ResponseModel) -> Element<'static, Message> {
    let mut block = String::new();
    for (name, value) in resp.headers.iter() {
        block.push_str(name.as_str());
        block.push_str(": ");
        block.push_str(value.to_str().unwrap_or("<non-utf8>"));
        block.push('\n');
    }
    if block.is_empty() {
        block.push_str("<no headers>");
    }
    scrollable(text(block).size(12))
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn cookies_view(resp: &ResponseModel) -> Element<'static, Message> {
    let mut col = column![].spacing(6).width(Length::Fill);
    let mut any = false;
    for value in resp.headers.get_all("set-cookie").iter() {
        any = true;
        let raw = value.to_str().unwrap_or("<non-utf8>");
        let mut parts = raw.splitn(2, ';');
        let pair = parts.next().unwrap_or(raw);
        let attrs = parts.next().unwrap_or("").trim();
        let (name, val) = pair.split_once('=').unwrap_or((pair, ""));
        let mut entry =
            column![text(format!("{} = {}", name.trim(), val.trim())).size(13),].spacing(2);
        if !attrs.is_empty() {
            entry = entry.push(text(format!("    {attrs}")).size(11));
        }
        col = col.push(entry);
    }
    if !any {
        col = col.push(text("No Set-Cookie headers in the response.").size(13));
    }
    scrollable(col)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn preview_view(resp: &ResponseModel) -> Element<'static, Message> {
    let content_type = resp
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let body: Element<'static, Message> = if content_type.contains("image") {
        text(format!(
            "Image response ({content_type}). Inline image preview is not supported; \
             save the body to a file to view it."
        ))
        .size(13)
        .into()
    } else if content_type.contains("html") {
        // No HTML renderer available; show the markup source.
        scrollable(text(resp.body.clone()).size(12))
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    } else {
        scrollable(text(format_body(&resp.body, true)).size(12))
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    };
    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Best-effort body formatting: pretty-print JSON when enabled and parseable, else the raw body.
fn format_body(body: &str, pretty_json: bool) -> String {
    if body.is_empty() {
        return "<empty body>".to_string();
    }
    if pretty_json
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(body)
        && let Ok(pretty) = serde_json::to_string_pretty(&value)
    {
        return pretty;
    }
    body.to_string()
}

/// Filter `text` to lines containing `query` (case-insensitive). Returns the text to show plus an
/// optional match-count header. An empty query returns the text unchanged.
fn filter_search(text: &str, query: &str) -> (String, Option<String>) {
    let query = query.trim();
    if query.is_empty() {
        return (text.to_string(), None);
    }
    let needle = query.to_ascii_lowercase();
    let matches: Vec<&str> = text
        .lines()
        .filter(|line| line.to_ascii_lowercase().contains(&needle))
        .collect();
    let header = format!("{} matching line(s) for \"{query}\"", matches.len());
    (matches.join("\n"), Some(header))
}
