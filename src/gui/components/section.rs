/// A tiny, reusable "card-like" section wrapper for consistent layout.
///
/// This is intentionally lean: it standardizes spacing, padding, and an optional title row.
/// Styling is kept minimal and theme-friendly (defaults to Iced's theme).
///
/// Usage:
/// ```ignore
/// let content = column![...].spacing(8.0);
/// let section = Section::new("Headers", content).into_element();
/// ```
use iced::widget::{Column, Container, column, container, text};
use iced::{Element, Length};

use crate::gui::Message;

/// A simple section/card wrapper.
///
/// Note: `Element` is not `Debug`/`Clone`, so we do not derive those traits here.
pub struct Section<'a> {
    title: Option<&'a str>,
    body: Element<'a, Message>,
    padding: u16,
    spacing: f32,
    width: Length,
}

impl<'a> Section<'a> {
    /// Create a section with a title.
    pub fn new(title: &'a str, body: impl Into<Element<'a, Message>>) -> Self {
        Self {
            title: Some(title),
            body: body.into(),
            padding: 14,
            spacing: 10.0,
            width: Length::Fill,
        }
    }

    /// Create a section without a title.
    pub fn untitled(body: impl Into<Element<'a, Message>>) -> Self {
        Self {
            title: None,
            body: body.into(),
            padding: 14,
            spacing: 10.0,
            width: Length::Fill,
        }
    }

    /// Set padding (default: 14).
    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    /// Set internal spacing between title and body (default: 10.0).
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set width (default: `Length::Fill`).
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Build the `Element`.
    pub fn into_element(self) -> Element<'a, Message> {
        let mut col: Column<'a, Message> = column!().spacing(self.spacing).width(self.width);

        if let Some(title) = self.title {
            col = col.push(text(title).size(16));
        }

        col = col.push(self.body);

        // Use default theme styling; "card-like" feel comes from padding and grouping.
        // If you later want borders/background, add a custom style here.
        container(col)
            .padding(self.padding)
            .width(self.width)
            .into()
    }

    /// Convenience: returns the underlying container if you need to add extra modifiers.
    pub fn into_container(self) -> Container<'a, Message> {
        let mut col: Column<'a, Message> = column!().spacing(self.spacing).width(self.width);

        if let Some(title) = self.title {
            col = col.push(text(title).size(16));
        }

        col = col.push(self.body);

        container(col).padding(self.padding).width(self.width)
    }
}
