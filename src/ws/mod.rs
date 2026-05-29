//! Raw WebSocket sessions via tokio-tungstenite, surfaced to the GUI as an iced `Subscription`.
//!
//! On connect the subscription emits [`WsEvent::Connected`] carrying an mpsc sender the app uses
//! to push outbound messages; incoming frames arrive as [`WsEvent::Received`]. The connection
//! lives as long as the app keeps returning the subscription (keyed by [`WsConfig`]).

use std::time::Duration;

use iced::Subscription;
use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as Frame;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};

use crate::model::NodePath;

/// Maximum consecutive reconnect attempts before giving up.
const MAX_RECONNECTS: u32 = 8;

/// Connection parameters. `Hash`/`Eq` so the subscription identity is stable per connection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WsConfig {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub subprotocols: Vec<String>,
    /// Reconnect automatically (with backoff) after a transient drop.
    pub auto_reconnect: bool,
    /// Connection timeout in milliseconds (a connect that exceeds this is treated as a failure).
    pub connect_timeout_ms: u64,
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
    /// A transient drop occurred; reconnecting after backoff (carries the attempt number).
    Reconnecting(u32),
    Failed(String),
    Closed,
}

/// A subscription that connects to `config` and streams events tagged with `path`. The path is
/// part of both the subscription identity (so identical configs stay distinct) and each item (so
/// the app can route events without a capturing `Subscription::map` closure, which iced forbids).
pub fn connect(path: NodePath, config: WsConfig) -> Subscription<(NodePath, WsEvent)> {
    Subscription::run_with((path, config), build_stream)
}

type EventStream = std::pin::Pin<Box<dyn iced::futures::Stream<Item = (NodePath, WsEvent)> + Send>>;

fn build_stream(key: &(NodePath, WsConfig)) -> EventStream {
    let tag = key.0.clone();
    let config = key.1.clone();
    let events = iced::stream::channel(64, async move |mut output: mpsc::Sender<WsEvent>| {
        let mut attempt: u32 = 0;
        loop {
            let request = match build_request(&config) {
                Ok(r) => r,
                Err(e) => {
                    let _ = output.send(WsEvent::Failed(e)).await;
                    return;
                }
            };
            // Bound the connect so a blackholed SYN / stalled TLS handshake cannot hang forever.
            let timeout = Duration::from_millis(config.connect_timeout_ms.max(1));
            match tokio::time::timeout(timeout, connect_async(request)).await {
                Ok(Ok((stream, _resp))) => {
                    attempt = 0;
                    if !run_session(stream, &mut output, config.auto_reconnect).await {
                        let _ = output.send(WsEvent::Closed).await;
                        return;
                    }
                }
                Ok(Err(e)) if !config.auto_reconnect => {
                    let _ = output.send(WsEvent::Failed(e.to_string())).await;
                    return;
                }
                Err(_) if !config.auto_reconnect => {
                    let _ = output
                        .send(WsEvent::Failed("connect timed out".to_string()))
                        .await;
                    return;
                }
                // Connect failed or timed out, but auto-reconnect is on: fall through to backoff.
                Ok(Err(_)) | Err(_) => {}
            }
            // Reconnect path: back off, then retry up to the cap.
            attempt += 1;
            if attempt > MAX_RECONNECTS {
                let _ = output
                    .send(WsEvent::Failed(format!(
                        "gave up reconnecting after {MAX_RECONNECTS} attempts"
                    )))
                    .await;
                return;
            }
            if output.send(WsEvent::Reconnecting(attempt)).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(backoff_ms(attempt))).await;
        }
    });
    // Tag each event with the session path at the stream level (capturing is fine here; only
    // iced's `Subscription::map` forbids capturing closures).
    Box::pin(events.map(move |event| (tag.clone(), event)))
}

/// Run one connected session until it ends. Returns `true` when the caller should reconnect (a
/// transient drop while auto-reconnect is on), `false` for a clean/server/user-initiated close.
async fn run_session(
    stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    output: &mut mpsc::Sender<WsEvent>,
    auto_reconnect: bool,
) -> bool {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<WsCommand>(64);
    if output.send(WsEvent::Connected(cmd_tx)).await.is_err() {
        return false;
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
                        if closed { return false; }
                    }
                }
                Some(Err(_)) => return auto_reconnect,
                None => return auto_reconnect,
            },
            command = cmd_rx.next() => match command {
                Some(WsCommand::Send(out)) => {
                    let frame = match out {
                        WsOutgoing::Text(t) => Frame::Text(t.into()),
                        WsOutgoing::Binary(b) => Frame::Binary(b.into()),
                    };
                    if write.send(frame).await.is_err() { return auto_reconnect; }
                }
                Some(WsCommand::Close) | None => {
                    let _ = write.send(Frame::Close(None)).await;
                    return false;
                }
            },
        }
    }
}

/// Exponential backoff (500ms · 2^(n-1)), capped at 10s.
fn backoff_ms(attempt: u32) -> u64 {
    let base = 500u64.saturating_mul(1u64 << attempt.saturating_sub(1).min(5));
    base.min(10_000)
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
