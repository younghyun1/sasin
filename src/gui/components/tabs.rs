//! Tab bar: one underline-style entry per open request, with dirty marker and close affordance.

use iced::alignment::Vertical;
use iced::widget::button;
use iced::widget::{row, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::components::tab_strip;
use crate::gui::state::Tab;
use crate::gui::theme;

/// Render the tab strip. Returns an empty row when nothing is open.
pub fn view(tabs: &[Tab], active: Option<usize>) -> Element<'static, Message> {
    let mut items: Vec<Element<'static, Message>> = Vec::new();
    for (i, tab) in tabs.iter().enumerate() {
        let is_active = active == Some(i);
        let mut label = row![text(tab.name.clone()).size(13)]
            .spacing(6)
            .align_y(Vertical::Center);
        if tab.dirty {
            label = label.push(text("•").size(13).style(|theme: &iced::Theme| {
                iced::widget::text::Style {
                    color: Some(theme.extended_palette().primary.base.color),
                }
            }));
        }
        let select = tab_strip::tab(label, is_active, Message::SelectTab(i));
        let close = button(theme::icons::icon(theme::icons::X, 11.0))
            .padding(6)
            .style(theme::flat)
            .on_press(Message::CloseTab(i));
        items.push(
            row![select, close]
                .spacing(2)
                .align_y(Vertical::Center)
                .into(),
        );
    }
    row(items).spacing(4).width(Length::Fill).into()
}
