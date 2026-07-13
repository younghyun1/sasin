//! Sidebar request-history list: the most recent sends, newest first. Clicking an entry re-creates
//! the request (method + url) and opens it.

use iced::alignment::Vertical;
use iced::widget::{button, column, row, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::theme;
use crate::storage::HistoryCache;

/// How many recent records to show.
const SHOWN: usize = 10;

/// Render the history section from the persisted cache.
pub fn view(history: &HistoryCache) -> Element<'static, Message> {
    let mut col = column![text("HISTORY").size(11).style(theme::muted)]
        .spacing(4)
        .width(Length::Fill);
    if history.records.is_empty() {
        col = col.push(text("No requests sent yet.").size(12));
        return col.into();
    }
    // Records are stored newest-last; show the tail, newest first. Pass the record by value so the
    // click is self-contained (a positional index could shift if the cap-drain runs meanwhile).
    for record in history.records.iter().rev().take(SHOWN) {
        let method = record.method.clone();
        let badge = text(method.clone())
            .size(10)
            .font(theme::fonts::MONO)
            .width(Length::Fixed(theme::metrics::BADGE_W))
            .style(move |t: &iced::Theme| iced::widget::text::Style {
                color: Some(theme::method_color(&method, t)),
            });
        let label = text(truncate(&record.url, 36)).size(12);
        col = col.push(
            button(row![badge, label].spacing(6).align_y(Vertical::Center))
                .padding(4)
                .width(Length::Fill)
                .style(theme::flat)
                .on_press(Message::HistoryOpen(record.clone())),
        );
    }
    col.into()
}

/// Truncate a string to `max` chars with an ellipsis (char-boundary safe).
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}
