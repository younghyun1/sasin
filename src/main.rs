use iced::{
    widget::{button, column, text, text_input, Column},
    Task, Theme,
};
use reqwest;
use std::result::Result as StdResult;

#[derive(Default)]
struct PostmanClone {
    url: String,
    response: String,
    client: reqwest::Client,
}

#[derive(Debug, Clone)]
enum Message {
    UrlChanged(String),
    SendPressed,
    ResponseReceived(String),
    ErrorReceived(String),
}

fn update(state: &mut PostmanClone, message: Message) -> Task<Message> {
    match message {
        Message::UrlChanged(new_url) => {
            state.url = new_url;
            Task::none()
        }
        Message::SendPressed => {
            let url = state.url.clone();
            let client = state.client.clone();
            Task::perform(
                async move {
                    match fetch_url(url, client).await {
                        Ok(body) => Message::ResponseReceived(body),
                        Err(e) => Message::ErrorReceived(format!("Error: {:?}", e)),
                    }
                },
                |msg| msg,
            )
        }
        Message::ResponseReceived(body) => {
            state.response = body;
            Task::none()
        }
        Message::ErrorReceived(error_message) => {
            state.response = error_message;
            Task::none()
        }
    }
}

fn view(state: &PostmanClone) -> Column<Message> {
    column![
        text("Enter URL:"),
        text_input(
            "https://httpbin.org/get", // placeholder text
            &state.url,
        )
        .on_input(|s| Message::UrlChanged(s))
        .padding(10),
        button("Send").padding(10).on_press(Message::SendPressed),
        text("Response:"),
        text(&state.response).size(16),
    ]
    .padding(20)
    .spacing(10)
}

async fn fetch_url(url: String, client: reqwest::Client) -> StdResult<String, reqwest::Error> {
    let response = client.get(&url).send().await?;

    let body = response.text().await?;

    Ok(body)
}

pub fn main() -> iced::Result {
    iced::application("Postman Clone", update, view)
        .theme(theme)
        .run()
}

fn theme(_state: &PostmanClone) -> Theme {
    Theme::CatppuccinMocha
}
