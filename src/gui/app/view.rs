//! Rendering for [`App`]: sidebar tree, tab bar + editor, response, in two resizable splits.

use iced::widget::{Space, button, column, container, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::components::{ResponseView, Split, SplitAxis, editor, tabs, tree};
use crate::gui::messages::SplitId;
use crate::gui::theme;
use crate::model::{Node, find_node};

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        let selected = self.active.and_then(|i| self.tabs.get(i)).map(|t| &t.path);
        let sidebar = container(
            column![
                tree::view(&self.workspace, &self.expanded, selected),
                button(text("New Request").size(13))
                    .padding(8)
                    .on_press(Message::NewRequest),
            ]
            .spacing(10)
            .padding(10),
        )
        .style(theme::panel)
        .width(Length::Fill)
        .height(Length::Fill);

        let tab_bar = tabs::view(&self.tabs, self.active);
        let active_tab = self.active.and_then(|i| self.tabs.get(i));

        let editor_area: Element<'_, Message> = match active_tab {
            Some(tab) => {
                let panel: Element<'_, Message> = match find_node(&self.workspace.root, &tab.path) {
                    Some(Node::Http(req)) => editor::view(req, tab),
                    Some(Node::Ws(_)) => container(
                        text("WebSocket sessions are interactive — console arrives in P7.")
                            .size(14),
                    )
                    .padding(20)
                    .center_x(Length::Fill)
                    .into(),
                    _ => container(text("Not editable.").size(14)).padding(20).into(),
                };
                column![tab_bar, panel].spacing(6).into()
            }
            None => column![
                tab_bar,
                container(text("Open a request from the tree, or create one.").size(16))
                    .padding(24)
                    .center_x(Length::Fill),
            ]
            .spacing(6)
            .into(),
        };

        let response = ResponseView::new()
            .response(active_tab.and_then(|t| t.response.as_ref()))
            .error(active_tab.and_then(|t| t.error.as_deref()))
            .show_headers(self.show_headers)
            .pretty_json(self.pretty_json)
            .body_text(None)
            .headers_height(Length::Fixed(160.0))
            .view();

        let main: Element<'_, Message> = Split::new(SplitAxis::Vertical)
            .first(editor_area)
            .second(response)
            .split_px(self.editor_px)
            .min_first_px(220.0)
            .min_second_px(160.0)
            .on_drag(|px| Message::SplitDragged(SplitId::RequestResponse, px))
            .into();

        let content: Element<'_, Message> = Split::new(SplitAxis::Horizontal)
            .first(sidebar)
            .second(main)
            .split_px(self.sidebar_px)
            .min_first_px(220.0)
            .min_second_px(420.0)
            .on_drag(|px| Message::SplitDragged(SplitId::Sidebar, px))
            .into();

        let status: Element<'_, Message> = match &self.status {
            Some(s) => text(s).size(12).into(),
            None => Space::new().height(Length::Fixed(0.0)).into(),
        };

        container(column![content, status].spacing(4))
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
