//! Sidebar request-history list: the most recent sends, newest first, with a filter box,
//! a show-more affordance, and a clear action. Clicking an entry re-creates the request
//! (method + url) and opens it.

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, row, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::theme;
use crate::storage::HistoryCache;

/// Render the history section from the persisted cache. `filter` narrows by method/url;
/// `shown` caps the visible rows (grown via the "more" button).
pub fn view(history: &HistoryCache, filter: &str, shown: usize) -> Element<'static, Message> {
    let header = row![
        text("HISTORY").size(11).style(theme::muted),
        Space::new().width(Length::Fill),
        button(text("clear").size(10).style(theme::muted))
            .padding(2)
            .style(theme::flat)
            .on_press_maybe((!history.records.is_empty()).then_some(Message::HistoryClear)),
    ]
    .align_y(Vertical::Center);
    let mut col = column![header].spacing(4).width(Length::Fill);
    if history.records.is_empty() {
        col = col.push(text("No requests sent yet.").size(12));
        return col.into();
    }

    col = col.push(
        text_input("filter history…", filter)
            .on_input(Message::HistoryFilterChanged)
            .padding(4)
            .size(12)
            .width(Length::Fill),
    );

    let needle = filter.trim().to_ascii_lowercase();
    // Records are stored newest-last; show the tail, newest first. Pass the record by value so the
    // click is self-contained (a positional index could shift if the cap-drain runs meanwhile).
    let matching: Vec<_> = history
        .records
        .iter()
        .rev()
        .filter(|r| {
            needle.is_empty()
                || r.method.to_ascii_lowercase().contains(&needle)
                || r.url.to_ascii_lowercase().contains(&needle)
        })
        .collect();

    for record in matching.iter().take(shown) {
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
                .on_press(Message::HistoryOpen((*record).clone())),
        );
    }
    if matching.len() > shown {
        col = col.push(
            button(
                text(format!("{} more…", matching.len() - shown))
                    .size(11)
                    .style(theme::muted),
            )
            .padding(4)
            .style(theme::flat)
            .on_press(Message::HistoryShowMore),
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
