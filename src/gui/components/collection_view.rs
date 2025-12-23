use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::persist::{Dataset, DatasetId};

#[derive(Debug, Clone)]
pub struct CollectionView<'a> {
    dataset: &'a Dataset,
    selected_request: Option<DatasetId>,
    height: Length,
}

impl<'a> CollectionView<'a> {
    pub fn new(dataset: &'a Dataset) -> Self {
        Self {
            dataset,
            selected_request: None,
            height: Length::Fill,
        }
    }

    pub fn selected(mut self, selected: Option<DatasetId>) -> Self {
        self.selected_request = selected;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn view(self) -> Element<'a, Message> {
        if self.dataset.collections.is_empty() {
            return text("No collections yet.").size(14).into();
        }

        let mut col = column!().spacing(8).width(Length::Fill);

        for collection in &self.dataset.collections {
            col = col.push(text(&collection.name).size(16)); // Collection name

            let mut requests_col = column!().spacing(4).padding(20.0); // Indent requests

            for r in &collection.requests {
                let label = format!("{} • {} {}", r.name, r.method.as_str(), r.url);

                // Main select button
                let mut select_btn = button(text(label).size(13)).padding(10);

                if self.selected_request != Some(r.id) {
                    select_btn = select_btn.on_press(Message::RequestSelected(r.id));
                }

                // Delete button
                let del_btn = button(text("Del").size(12))
                    .padding(8)
                    .on_press(Message::DeleteRequestPressed(r.id));

                requests_col = requests_col.push(row![select_btn, del_btn].spacing(8));
            }
            col = col.push(requests_col);
        }

        scrollable(col)
            .height(self.height)
            .width(Length::Fill)
            .into()
    }
}
