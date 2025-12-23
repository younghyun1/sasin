use iced::widget::{button, column, scrollable, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::models::HttpMethod;

/// A lightweight, scrollable list of recent requests.
///
/// This is intentionally minimal and fast:
/// - One button per entry (select to restore into editor)
/// - No heavy formatting or stateful widgets
///
/// Wrap it in a `Section` for a card-like layout.
#[derive(Debug, Clone)]
pub struct HistoryList<'a> {
    items: &'a [HistoryItem],
    selected: Option<usize>,
    height: Length,
}

impl<'a> HistoryList<'a> {
    pub fn new(items: &'a [HistoryItem]) -> Self {
        Self {
            items,
            selected: None,
            height: Length::Fixed(200.0),
        }
    }

    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn view(self) -> Element<'a, Message> {
        if self.items.is_empty() {
            return text("No history yet.").size(14).into();
        }

        let mut col = column!().spacing(8).width(Length::Fill);

        for (idx, item) in self.items.iter().enumerate() {
            let label = format!("{} {}", item.method.as_str(), item.url);

            let mut b = button(text(label).size(13)).padding(10);

            // If already selected, make it a no-op (avoid redundant updates).
            if self.selected != Some(idx) {
                b = b.on_press(Message::HistorySelected(idx));
            }

            col = col.push(b);
        }

        scrollable(col)
            .height(self.height)
            .width(Length::Fill)
            .into()
    }
}

/// A compact history item, suitable for rendering quickly.
#[derive(Debug, Clone)]
pub struct HistoryItem {
    pub method: HttpMethod,
    pub url: String,
}

impl HistoryItem {
    pub fn new(method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
        }
    }
}
