//! Raw WebSocket sessions via tokio-tungstenite, surfaced to the GUI as an iced `Subscription`.
//!
//! On connect the subscription emits [`WsEvent::Connected`] carrying an mpsc sender the app uses
//! to push outbound messages; incoming frames arrive as [`WsEvent::Received`]. The connection
//! lives as long as the app keeps returning the subscription (keyed by [`WsConfig`]).

use iced::Subscription;
use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as Frame;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};

/// Connection parameters. `Hash`/`Eq` so the subscription identity is stable per connection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WsConfig {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub subprotocols: Vec<String>,
}

/// A message to send on an open connection.
#[derive(Debug, Clone)]
pub enum WsOutgoing {
    Text(String),
    Binary(Vec<u8>),
}

/// Commands the app sends into the connection task.
#[derive(Debug, Clone)]
pub enum WsCommand {
    Send(WsOutgoing),
    Close,
}

/// A decoded incoming frame (binary shown as a byte count).
#[derive(Debug, Clone)]
pub enum WsIncoming {
    Text(String),
    Binary(usize),
    Ping,
    Pong,
    Close,
}

/// Events the subscription emits to the app.
#[derive(Debug, Clone)]
pub enum WsEvent {
    Connected(mpsc::Sender<WsCommand>),
    Received(WsIncoming),
    Failed(String),
    Closed,
}

/// A subscription that connects to `config` and streams [`WsEvent`]s.
pub fn connect(config: &WsConfig) -> Subscription<WsEvent> {
    Subscription::run_with(config.clone(), build_stream)
}

type EventStream = std::pin::Pin<Box<dyn iced::futures::Stream<Item = WsEvent> + Send>>;

fn build_stream(config: &WsConfig) -> EventStream {
    let config = config.clone();
    Box::pin(iced::stream::channel(
        64,
        async move |mut output: mpsc::Sender<WsEvent>| {
            let request = match build_request(&config) {
                Ok(r) => r,
                Err(e) => {
                    let _ = output.send(WsEvent::Failed(e)).await;
                    return;
                }
            };

            let stream = match connect_async(request).await {
                Ok((stream, _resp)) => stream,
                Err(e) => {
                    let _ = output.send(WsEvent::Failed(e.to_string())).await;
                    return;
                }
            };

            let (cmd_tx, mut cmd_rx) = mpsc::channel::<WsCommand>(64);
            if output.send(WsEvent::Connected(cmd_tx)).await.is_err() {
                return;
            }

            let (mut write, read) = stream.split();
            let mut read = read.fuse();
            loop {
                iced::futures::select! {
                    incoming = read.next() => match incoming {
                        Some(Ok(frame)) => {
                            if let Some(msg) = decode(frame) {
                                let closed = matches!(msg, WsIncoming::Close);
                                let _ = output.send(WsEvent::Received(msg)).await;
                                if closed { break; }
                            }
                        }
                        Some(Err(e)) => { let _ = output.send(WsEvent::Failed(e.to_string())).await; break; }
                        None => break,
                    },
                    command = cmd_rx.next() => match command {
                        Some(WsCommand::Send(out)) => {
                            let frame = match out {
                                WsOutgoing::Text(t) => Frame::Text(t.into()),
                                WsOutgoing::Binary(b) => Frame::Binary(b.into()),
                            };
                            if write.send(frame).await.is_err() { break; }
                        }
                        Some(WsCommand::Close) | None => {
                            let _ = write.send(Frame::Close(None)).await;
                            break;
                        }
                    },
                }
            }
            let _ = output.send(WsEvent::Closed).await;
        },
    ))
}

fn build_request(
    config: &WsConfig,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>, String> {
    let mut request = config
        .url
        .as_str()
        .into_client_request()
        .map_err(|e| format!("invalid websocket url: {e}"))?;
    let headers = request.headers_mut();
    for (k, v) in &config.headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(k.trim().as_bytes()),
            HeaderValue::from_str(v.trim()),
        ) {
            headers.insert(name, value);
        }
    }
    if !config.subprotocols.is_empty()
        && let Ok(value) = HeaderValue::from_str(&config.subprotocols.join(", "))
    {
        headers.insert("sec-websocket-protocol", value);
    }
    Ok(request)
}

fn decode(frame: Frame) -> Option<WsIncoming> {
    match frame {
        Frame::Text(t) => Some(WsIncoming::Text(t.to_string())),
        Frame::Binary(b) => Some(WsIncoming::Binary(b.len())),
        Frame::Ping(_) => Some(WsIncoming::Ping),
        Frame::Pong(_) => Some(WsIncoming::Pong),
        Frame::Close(_) => Some(WsIncoming::Close),
        Frame::Frame(_) => None,
    }
}
