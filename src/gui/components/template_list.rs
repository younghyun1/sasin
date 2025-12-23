/// Template list component for dataset templates.
///
/// This component renders a scrollable list of saved request templates and
/// provides simple interactions:
/// - select a template
/// - delete a template
///
/// Kept intentionally lean; styling comes mostly from spacing and padding.
/// You can wrap this in a `Section` for a card-like layout.
use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::persist::{Dataset, DatasetId};

#[derive(Debug, Clone)]
pub struct TemplateList<'a> {
    dataset: &'a Dataset,
    selected: Option<DatasetId>,
    height: Length,
}

impl<'a> TemplateList<'a> {
    pub fn new(dataset: &'a Dataset) -> Self {
        Self {
            dataset,
            selected: None,
            height: Length::Fixed(220.0),
        }
    }

    pub fn selected(mut self, selected: Option<DatasetId>) -> Self {
        self.selected = selected;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn view(self) -> Element<'a, Message> {
        if self.dataset.templates.is_empty() {
            return text("No templates saved yet.").size(14).into();
        }

        let mut col = column!().spacing(8).width(Length::Fill);

        for t in &self.dataset.templates {
            let label = format!("{} • {} {}", t.name, t.method.as_str(), t.url);

            // Main select button
            let mut select_btn = button(text(label).size(13)).padding(10);

            if self.selected != Some(t.id) {
                select_btn = select_btn.on_press(Message::TemplateSelected(t.id));
            }

            // Delete button
            let del_btn = button(text("Del").size(12))
                .padding(8)
                .on_press(Message::DeleteTemplatePressed(t.id));

            col = col.push(row![select_btn, del_btn].spacing(8));
        }

        scrollable(col)
            .height(self.height)
            .width(Length::Fill)
            .into()
    }
}
