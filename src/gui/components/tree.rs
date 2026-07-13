//! Recursive collection-tree sidebar built from composable widgets (no custom Widget impl).
//!
//! Folders toggle expand/collapse; request/websocket leaves open in a tab. Rows are owned
//! (all strings cloned), so the returned `Element` borrows nothing from the workspace.
//! A row being renamed swaps its label for an inline text input.

use std::collections::HashSet;

use iced::widget::{Space, button, column, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::messages::{MoveDir, TreeMsg};
use crate::gui::theme;
use crate::gui::theme::icons;
use crate::model::{Node, NodePath, Workspace};

const INDENT_PX: f32 = 14.0;
const ROW_ICON: f32 = 11.0;

/// Build the scrollable tree view. `renaming` is the in-flight rename (path + edit buffer).
pub fn view(
    ws: &Workspace,
    expanded: &HashSet<NodePath>,
    selected: Option<&NodePath>,
    renaming: Option<&(NodePath, String)>,
) -> Element<'static, Message> {
    let mut rows: Vec<Element<'static, Message>> = Vec::new();
    let mut path: NodePath = Vec::new();
    collect(
        &mut rows, &ws.root, &mut path, 0, expanded, selected, renaming,
    );

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
    renaming: Option<&(NodePath, String)>,
) {
    for node in nodes {
        path.push(node.slug().to_string());
        let is_selected = selected.map(|s| s.as_slice()) == Some(path.as_slice());
        if let Some((rpath, buf)) = renaming
            && rpath == path
        {
            out.push(rename_row(buf, depth));
            if let Node::Folder(folder) = node
                && expanded.contains(path)
            {
                collect(
                    out,
                    &folder.children,
                    path,
                    depth + 1,
                    expanded,
                    selected,
                    renaming,
                );
            }
            path.pop();
            continue;
        }
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
                    collect(
                        out,
                        &folder.children,
                        path,
                        depth + 1,
                        expanded,
                        selected,
                        renaming,
                    );
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

/// Small flat icon button shared by all row actions.
fn action(glyph: char, msg: Message) -> Element<'static, Message> {
    button(theme::icons::icon(glyph, ROW_ICON).style(theme::muted))
        .padding(4)
        .style(theme::flat)
        .on_press(msg)
        .into()
}

/// The inline rename editor shown in place of a row's label.
fn rename_row(buf: &str, depth: u16) -> Element<'static, Message> {
    let input = text_input("name", buf)
        .on_input(|s| Message::Tree(TreeMsg::RenameInput(s)))
        .on_submit(Message::Tree(TreeMsg::RenameCommit))
        .padding(4)
        .size(13)
        .width(Length::Fill);
    row![indent(depth), input].spacing(4).into()
}

fn folder_row(
    name: &str,
    slug: &str,
    path: NodePath,
    depth: u16,
    expanded: bool,
) -> Element<'static, Message> {
    let arrow = if expanded {
        icons::CHEVRON_DOWN
    } else {
        icons::CHEVRON_RIGHT
    };
    let label = button(
        row![
            theme::icons::icon(arrow, 12.0),
            text(display(name, slug)).size(14)
        ]
        .spacing(8),
    )
    .padding(6)
    .width(Length::Fill)
    .style(theme::flat)
    .on_press(Message::ToggleFolder(path.clone()));

    row![
        indent(depth),
        label,
        action(
            icons::PLUS,
            Message::Tree(TreeMsg::NewRequestIn(path.clone()))
        ),
        action(
            icons::FOLDER_PLUS,
            Message::Tree(TreeMsg::NewFolder(path.clone()))
        ),
        action(
            icons::PENCIL,
            Message::Tree(TreeMsg::RenameStart(path.clone()))
        ),
        action(
            icons::ARROW_UP,
            Message::Tree(TreeMsg::Move(path.clone(), MoveDir::Up))
        ),
        action(
            icons::ARROW_DOWN,
            Message::Tree(TreeMsg::Move(path.clone(), MoveDir::Down))
        ),
        action(icons::PLAY, Message::OpenRunner(path)),
    ]
    .spacing(2)
    .into()
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
            theme::tree_row_selected
        } else {
            theme::flat
        })
        .on_press(Message::OpenNode(path.clone()));

    row![
        indent(depth),
        open,
        action(
            icons::PENCIL,
            Message::Tree(TreeMsg::RenameStart(path.clone()))
        ),
        action(icons::COPY, Message::Tree(TreeMsg::Duplicate(path.clone()))),
        action(
            icons::ARROW_UP,
            Message::Tree(TreeMsg::Move(path.clone(), MoveDir::Up))
        ),
        action(
            icons::ARROW_DOWN,
            Message::Tree(TreeMsg::Move(path.clone(), MoveDir::Down))
        ),
        action(icons::X, Message::DeleteNode(path)),
    ]
    .spacing(2)
    .into()
}

fn method_badge(method: &str) -> Element<'static, Message> {
    let m = method.to_string();
    text(m.clone())
        .size(10)
        .font(theme::fonts::MONO)
        .width(Length::Fixed(theme::metrics::BADGE_W))
        .style(move |theme| iced::widget::text::Style {
            color: Some(theme::method_color(&m, theme)),
        })
        .into()
}
