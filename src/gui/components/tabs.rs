//! Tab bar: one entry per open request, active tab highlighted, with a close affordance.

use iced::widget::{button, row, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::state::Tab;
use crate::gui::theme;

/// Render the tab strip. Returns an empty row when nothing is open.
pub fn view(tabs: &[Tab], active: Option<usize>) -> Element<'static, Message> {
    let mut items: Vec<Element<'static, Message>> = Vec::new();
    for (i, tab) in tabs.iter().enumerate() {
        let is_active = active == Some(i);
        let dot = if tab.dirty { "• " } else { "" };
        let title = format!("{dot}{}", tab.name);
        let select = button(text(title).size(13))
            .padding(8)
            .style(if is_active {
                theme::selected
            } else {
                theme::flat
            })
            .on_press(Message::SelectTab(i));
        let close = button(text("✕").size(11))
            .padding(6)
            .style(theme::flat)
            .on_press(Message::CloseTab(i));
        items.push(row![select, close].spacing(2).into());
    }
    row(items).spacing(6).width(Length::Fill).into()
}
