//! Recursive collection-tree sidebar built from composable widgets (no custom Widget impl).
//!
//! Folders toggle expand/collapse; request/websocket leaves open in a tab. Rows are owned
//! (all strings cloned), so the returned `Element` borrows nothing from the workspace.

use std::collections::HashSet;

use iced::widget::{Space, button, column, row, scrollable, text};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::theme;
use crate::model::{Node, NodePath, Workspace};

const INDENT_PX: f32 = 14.0;

/// Build the scrollable tree view.
pub fn view(
    ws: &Workspace,
    expanded: &HashSet<NodePath>,
    selected: Option<&NodePath>,
) -> Element<'static, Message> {
    let mut rows: Vec<Element<'static, Message>> = Vec::new();
    let mut path: NodePath = Vec::new();
    collect(&mut rows, &ws.root, &mut path, 0, expanded, selected);

    if rows.is_empty() {
        rows.push(
            text("No requests yet. Use \"New Request\".")
                .size(13)
                .into(),
        );
    }

    scrollable(column(rows).spacing(2).width(Length::Fill))
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn collect(
    out: &mut Vec<Element<'static, Message>>,
    nodes: &[Node],
    path: &mut NodePath,
    depth: u16,
    expanded: &HashSet<NodePath>,
    selected: Option<&NodePath>,
) {
    for node in nodes {
        path.push(node.slug().to_string());
        let is_selected = selected.map(|s| s.as_slice()) == Some(path.as_slice());
        match node {
            Node::Folder(folder) => {
                let is_expanded = expanded.contains(path);
                out.push(folder_row(
                    &folder.name,
                    &folder.slug,
                    path.clone(),
                    depth,
                    is_expanded,
                ));
                if is_expanded {
                    collect(out, &folder.children, path, depth + 1, expanded, selected);
                }
            }
            Node::Http(r) => out.push(leaf_row(
                &r.method,
                &r.name,
                &r.slug,
                path.clone(),
                depth,
                is_selected,
            )),
            Node::Ws(w) => out.push(leaf_row(
                "WS",
                &w.name,
                &w.slug,
                path.clone(),
                depth,
                is_selected,
            )),
        }
        path.pop();
    }
}

fn indent(depth: u16) -> Space {
    Space::new().width(Length::Fixed(f32::from(depth) * INDENT_PX))
}

fn display(name: &str, slug: &str) -> String {
    if name.is_empty() {
        slug.to_string()
    } else {
        name.to_string()
    }
}

fn folder_row(
    name: &str,
    slug: &str,
    path: NodePath,
    depth: u16,
    expanded: bool,
) -> Element<'static, Message> {
    let arrow = if expanded { "▾" } else { "▸" };
    let label = button(row![text(arrow).size(13), text(display(name, slug)).size(14)].spacing(8))
        .padding(6)
        .width(Length::Fill)
        .style(theme::flat)
        .on_press(Message::ToggleFolder(path.clone()));

    // Run the folder as a collection.
    let run = button(text("▶").size(11))
        .padding(4)
        .style(theme::flat)
        .on_press(Message::OpenRunner(path));

    row![indent(depth), label, run].spacing(4).into()
}

fn leaf_row(
    method: &str,
    name: &str,
    slug: &str,
    path: NodePath,
    depth: u16,
    selected: bool,
) -> Element<'static, Message> {
    let open = button(row![method_badge(method), text(display(name, slug)).size(14)].spacing(8))
        .padding(6)
        .width(Length::Fill)
        .style(if selected {
            theme::selected
        } else {
            theme::flat
        })
        .on_press(Message::OpenNode(path.clone()));

    let delete = button(text("✕").size(12))
        .padding(4)
        .style(theme::flat)
        .on_press(Message::DeleteNode(path));

    row![indent(depth), open, delete].spacing(4).into()
}

fn method_badge(method: &str) -> Element<'static, Message> {
    let m = method.to_string();
    text(m.clone())
        .size(10)
        .font(theme::fonts::MONO)
        .style(move |theme| iced::widget::text::Style {
            color: Some(theme::method_color(&m, theme)),
        })
        .into()
}
