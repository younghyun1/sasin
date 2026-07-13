//! Shared underline-style tab affordance used by the tab bar, the editor panel bar, and the
//! response sub-tabs: a flat label button over an accent rule that lights up when active.

use iced::widget::{Space, button, column, container};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::theme;

/// Wrap `label` as a tab that emits `on_press`; `active` drives the underline.
pub fn tab<'a>(
    label: impl Into<Element<'a, Message>>,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let btn = button(label.into())
        .padding([6, 10])
        .style(theme::flat)
        .on_press(on_press);
    let rule = container(
        Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(theme::metrics::UNDERLINE_H)),
    )
    .style(theme::underline(active));
    column![btn, rule].into()
}
