//! Flat search results for the sidebar: shown instead of the tree while the filter is
//! non-empty. Matches request/websocket leaves on name, slug path, method, and URL.

use iced::alignment::Vertical;
use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::theme;
use crate::storage::{IndexEntry, KIND_FOLDER};

/// Render the match list for `filter` over the flattened index.
pub fn view(entries: &[IndexEntry], filter: &str) -> Element<'static, Message> {
    let mut col = column![].spacing(2).width(Length::Fill);
    let mut any = false;
    for entry in entries.iter().filter(|e| matches(e, filter)) {
        any = true;
        col = col.push(result_row(entry));
    }
    if !any {
        col = col.push(text(format!("No matches for \"{filter}\".")).size(12));
    }
    scrollable(col)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

/// Case-insensitive substring match over name, path (slugs), method, and url. Folders are
/// excluded: search targets things you can open.
fn matches(entry: &IndexEntry, filter: &str) -> bool {
    if entry.kind == KIND_FOLDER {
        return false;
    }
    let needle = filter.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return false;
    }
    [&entry.name, &entry.path, &entry.method, &entry.url]
        .iter()
        .any(|hay| hay.to_ascii_lowercase().contains(&needle))
}

fn result_row(entry: &IndexEntry) -> Element<'static, Message> {
    let method = if entry.method.is_empty() {
        "WS".to_string()
    } else {
        entry.method.clone()
    };
    let badge_method = method.clone();
    let badge = text(method)
        .size(10)
        .font(theme::fonts::MONO)
        .width(Length::Fixed(theme::metrics::BADGE_W))
        .style(move |t: &iced::Theme| iced::widget::text::Style {
            color: Some(theme::method_color(&badge_method, t)),
        });
    let name = if entry.name.is_empty() {
        entry
            .path
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string()
    } else {
        entry.name.clone()
    };
    let crumb = text(entry.path.clone()).size(10).style(theme::muted);
    let path: Vec<String> = entry.path.split('/').map(str::to_string).collect();
    button(
        row![badge, column![text(name).size(13), crumb].spacing(1)]
            .spacing(6)
            .align_y(Vertical::Center),
    )
    .padding(4)
    .width(Length::Fill)
    .style(theme::flat)
    .on_press(Message::OpenNode(path))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{KIND_HTTP, KIND_WS};

    fn entry(kind: u8, path: &str, name: &str, method: &str, url: &str) -> IndexEntry {
        IndexEntry {
            path: path.to_string(),
            kind,
            name: name.to_string(),
            method: method.to_string(),
            url: url.to_string(),
        }
    }

    #[test]
    fn matches_name_path_method_url_case_insensitive() {
        let e = entry(
            KIND_HTTP,
            "users/list",
            "List Users",
            "GET",
            "https://api.example.com/users",
        );
        for needle in ["list users", "users/", "get", "API.EXAMPLE", "users"] {
            assert!(matches(&e, needle), "{needle} should match");
        }
        assert!(!matches(&e, "delete"));
        assert!(!matches(&e, "  "));
    }

    #[test]
    fn folders_are_excluded() {
        let f = entry(KIND_FOLDER, "users", "Users", "", "");
        assert!(!matches(&f, "users"));
        let w = entry(KIND_WS, "chat", "Chat", "", "wss://x");
        assert!(matches(&w, "chat"));
    }
}
