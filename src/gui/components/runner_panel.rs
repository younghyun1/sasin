//! Collection-runner panel: configure iterations + data file, start/stop, and show a live summary.

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::gui::Message;
use crate::gui::runner_state::RunnerState;
use crate::gui::theme;
use crate::runner::RequestOutcome;

/// Render the runner panel for an active session.
pub fn view(runner: &RunnerState) -> Element<'_, Message> {
    let header = row![
        text(format!("Run — {}", runner.root_name)).size(18),
        Space::new().width(Length::Fill),
        button(text("Close").size(13))
            .padding(8)
            .on_press(Message::RunnerClose),
    ]
    .spacing(10)
    .align_y(Vertical::Center);

    let config = row![
        text("Iterations").size(13),
        text_input("1", &runner.iterations_text)
            .on_input(Message::RunnerIterations)
            .padding(6)
            .size(13)
            .width(Length::Fixed(70.0)),
        text("Data file").size(13),
        text_input("data.csv | data.json", &runner.data_path)
            .on_input(Message::RunnerDataPathChanged)
            .padding(6)
            .size(13)
            .width(Length::Fill),
        button(text("Load").size(13))
            .padding(6)
            .on_press(Message::RunnerLoadData),
    ]
    .spacing(8)
    .align_y(Vertical::Center);

    let action = if runner.running {
        button(text("Stop").size(14))
            .padding(10)
            .style(theme::selected)
            .on_press(Message::RunnerStop)
    } else {
        button(text(format!("Run {} request(s)", runner.requests.len())).size(14))
            .padding(10)
            .style(theme::selected)
            .on_press_maybe((!runner.requests.is_empty()).then_some(Message::RunnerStart))
    };

    let report = &runner.report;
    let state_suffix = if runner.finished {
        " • done"
    } else if runner.running {
        " • running…"
    } else {
        ""
    };
    let progress = text(format!(
        "{}/{} sent • {} passed • {} failed • assertions {}/{}{state_suffix}",
        report.requests(),
        runner.total(),
        report.passed_requests(),
        report.failed_requests(),
        report.passed_assertions(),
        report.total_assertions(),
    ))
    .size(13);

    let mut list = column![].spacing(3).width(Length::Fill);
    if report.outcomes.is_empty() {
        list = list.push(text("No results yet. Configure iterations / data, then Run.").size(13));
    }
    for outcome in &report.outcomes {
        list = list.push(outcome_row(outcome));
    }

    container(
        column![
            header,
            config,
            action,
            progress,
            scrollable(list).height(Length::Fill).width(Length::Fill),
        ]
        .spacing(10)
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .style(theme::panel)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn outcome_row(o: &RequestOutcome) -> Element<'_, Message> {
    let passed = o.passed();
    let mark = theme::icons::icon(
        if passed {
            theme::icons::CHECK
        } else {
            theme::icons::CIRCLE_X
        },
        12.0,
    )
    .style(move |t: &iced::Theme| {
        let p = t.extended_palette();
        iced::widget::text::Style {
            color: Some(if passed {
                p.success.base.color
            } else {
                p.danger.base.color
            }),
        }
    });
    let status = o
        .status
        .map(|c| c.to_string())
        .unwrap_or_else(|| "—".to_string());
    let head = row![
        mark,
        text(format!(
            "{} · iter {} · {status} · {} ms",
            o.name,
            o.iteration + 1,
            o.duration_ms
        ))
        .size(13)
    ]
    .spacing(6)
    .align_y(Vertical::Center);
    let mut col = column![head].spacing(2);
    if let Some(e) = &o.error {
        col = col.push(text(format!("    error: {e}")).size(11));
    }
    for t in &o.tests {
        if !t.passed {
            let detail = t.error.as_deref().unwrap_or("");
            col = col.push(text(format!("    ✗ {} {detail}", t.name)).size(11));
        }
    }
    col.into()
}
