//! Rendering for [`App`]: sidebar tree, tab bar + editor, response, in two resizable splits.

use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::components::{
    ResponseView, Split, SplitAxis, cookie_manager, editor, env_panel, history_panel, runner_panel,
    tabs, tree, ws_console,
};
use crate::gui::messages::SplitId;
use crate::gui::state::Tab;
use crate::gui::theme;
use crate::model::{Node, find_node};

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        let selected = self.active.and_then(|i| self.tabs.get(i)).map(|t| &t.path);
        let sidebar = container(
            column![
                tree::view(
                    &self.workspace,
                    &self.expanded,
                    selected,
                    self.renaming.as_ref()
                ),
                row![
                    button(text("New Request").size(13))
                        .padding(8)
                        .width(Length::Fill)
                        .on_press(Message::NewRequest),
                    button(theme::icons::icon(theme::icons::FOLDER_PLUS, 13.0))
                        .padding(8)
                        .on_press(Message::Tree(crate::gui::messages::TreeMsg::NewFolder(
                            Vec::new()
                        ))),
                    button(text("Run All").size(13))
                        .padding(8)
                        .on_press(Message::OpenRunner(Vec::new())),
                    button(text("Cookies").size(13))
                        .padding(8)
                        .on_press(Message::ToggleCookieManager),
                ]
                .spacing(6),
                env_panel::view(&self.workspace.environments, self.active_env),
                history_panel::view(&self.history),
                row![
                    text_input("paste curl…", &self.curl_import_text)
                        .on_input(Message::CurlImportChanged)
                        .padding(6)
                        .size(12)
                        .font(theme::fonts::MONO)
                        .width(Length::Fill),
                    button(text("Import").size(12))
                        .padding(6)
                        .on_press(Message::CurlImport),
                ]
                .spacing(6),
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
                    Some(Node::Http(req)) => {
                        editor::view(req, tab, theme::code_theme(self.prefs.theme))
                    }
                    Some(Node::Ws(req)) => {
                        let rt = self.ws.iter().find(|r| r.path == tab.path);
                        ws_console::view(req, rt)
                    }
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
            .tab(self.response_tab)
            .pretty_json(self.pretty_json)
            .search(&self.response_search)
            .view();

        let response_pane: Element<'_, Message> = match active_tab {
            Some(tab) => column![script_results(tab), response].spacing(4).into(),
            None => response,
        };

        // The runner / cookie-manager views take over the main area when open.
        let main: Element<'_, Message> = if self.show_cookies {
            cookie_manager::view(&self.http_config.jar.snapshot())
        } else if let Some(runner) = &self.runner {
            runner_panel::view(runner)
        } else {
            Split::new(SplitAxis::Vertical)
                .first(editor_area)
                .second(response_pane)
                .split_px(self.editor_px)
                .min_first_px(220.0)
                .min_second_px(160.0)
                .on_drag(|px| Message::SplitDragged(SplitId::RequestResponse, px))
                .into()
        };

        let content: Element<'_, Message> = Split::new(SplitAxis::Horizontal)
            .first(sidebar)
            .second(main)
            .split_px(self.sidebar_px)
            .min_first_px(220.0)
            .min_second_px(420.0)
            .on_drag(|px| Message::SplitDragged(SplitId::Sidebar, px))
            .into();

        container(column![content, self.status_bar()])
            .style(theme::surface)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Permanent bottom strip: status text left; active environment + theme toggle right.
    fn status_bar(&self) -> Element<'_, Message> {
        let status = text(self.status.as_deref().unwrap_or("")).size(12);
        let env_name = self
            .active_env
            .and_then(|i| self.workspace.environments.get(i))
            .map(|e| if e.name.is_empty() { &e.slug } else { &e.name })
            .cloned()
            .unwrap_or_else(|| "no environment".to_string());
        container(
            row![
                status,
                Space::new().width(Length::Fill),
                text(env_name).size(12).style(theme::muted),
                button(theme_icon(self.prefs.theme))
                    .padding(2)
                    .style(theme::flat)
                    .on_press(Message::ToggleTheme),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center),
        )
        .style(theme::status_bar)
        .padding([2, 10])
        .height(Length::Fixed(theme::metrics::STATUS_BAR_H))
        .width(Length::Fill)
        .into()
    }
}

/// Theme-toggle icon: shows the theme you would switch to.
fn theme_icon<'a>(choice: crate::persist::ThemeChoice) -> iced::widget::Text<'a> {
    let glyph = match choice {
        crate::persist::ThemeChoice::Dark => theme::icons::SUN,
        crate::persist::ThemeChoice::Light => theme::icons::MOON,
    };
    theme::icons::icon(glyph, 14.0)
}

/// A compact strip of test results + console output from the last script run (empty when none).
fn script_results(tab: &Tab) -> Element<'_, Message> {
    if tab.script_tests.is_empty() && tab.script_console.is_empty() && tab.script_error.is_none() {
        return Space::new().height(Length::Fixed(0.0)).into();
    }
    let mut col = column![text("Tests").size(13)].spacing(2);
    if let Some(e) = &tab.script_error {
        col = col.push(text(format!("⚠ script error: {e}")).size(12));
    }
    for t in &tab.script_tests {
        let line = if t.passed {
            format!("✓ {}", t.name)
        } else {
            match &t.error {
                Some(e) => format!("✗ {} — {e}", t.name),
                None => format!("✗ {}", t.name),
            }
        };
        col = col.push(text(line).size(12));
    }
    for line in &tab.script_console {
        col = col.push(text(format!("» {line}")).size(11));
    }
    container(scrollable(col).height(Length::Fixed(120.0)))
        .padding(6)
        .width(Length::Fill)
        .into()
}
