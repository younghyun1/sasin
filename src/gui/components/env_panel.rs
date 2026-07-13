//! Environment selector + variable editor for the sidebar.

use iced::widget::{button, checkbox, column, row, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::messages::KvOp;
use crate::gui::theme;
use crate::model::{Environment, Variable};

/// Render the environment selector (one button per env + "+ Env") and the active env's variables.
pub fn view(envs: &[Environment], active: Option<usize>) -> Element<'_, Message> {
    let mut selector: Vec<Element<'_, Message>> = Vec::new();
    for (i, env) in envs.iter().enumerate() {
        let label = if env.name.is_empty() {
            &env.slug
        } else {
            &env.name
        };
        let btn = button(text(label.clone()).size(12))
            .padding(6)
            .on_press(Message::SelectEnv(i));
        selector.push(
            if active == Some(i) {
                btn.style(theme::selected)
            } else {
                btn.style(theme::flat)
            }
            .into(),
        );
    }
    selector.push(
        button(text("+ Env").size(12))
            .padding(6)
            .style(theme::flat)
            .on_press(Message::NewEnv)
            .into(),
    );

    let vars: Element<'_, Message> = match active.and_then(|i| envs.get(i)) {
        Some(env) => variables_table(&env.variables),
        None => text("No environment.").size(12).into(),
    };

    column![
        text("ENVIRONMENT").size(11).style(theme::muted),
        row(selector).spacing(4),
        vars
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

fn variables_table(vars: &[Variable]) -> Element<'_, Message> {
    let mut rows: Vec<Element<'_, Message>> = Vec::new();
    for (i, v) in vars.iter().enumerate() {
        let enabled = checkbox(v.enabled).on_toggle(move |b| Message::EnvVar(KvOp::Toggle(i, b)));
        let key = text_input("key", &v.key)
            .on_input(move |s| Message::EnvVar(KvOp::Key(i, s)))
            .padding(4)
            .size(12)
            .width(Length::FillPortion(2));
        let value = text_input("value", &v.value)
            .on_input(move |s| Message::EnvVar(KvOp::Value(i, s)))
            .padding(4)
            .size(12)
            .width(Length::FillPortion(3));
        let delete = button(theme::icons::icon(theme::icons::TRASH, 11.0).style(theme::muted))
            .padding(4)
            .style(theme::flat)
            .on_press(Message::EnvVar(KvOp::Remove(i)));
        rows.push(row![enabled, key, value, delete].spacing(4).into());
    }
    rows.push(
        button(text("+ var").size(11))
            .padding(4)
            .style(theme::flat)
            .on_press(Message::EnvVar(KvOp::Add))
            .into(),
    );
    column(rows).spacing(4).width(Length::Fill).into()
}
