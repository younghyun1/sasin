//! WebSocket session handling: the connection subscription, config building, and message routing.

use iced::Subscription;

use crate::gui::Message;
use crate::gui::app::App;
use crate::gui::state::{WsDir, WsRuntime};
use crate::model::{ApiKeyLoc, Auth, Node, WsKind, WsRequest, find_node, resolve_auth};
use crate::runtime;
use crate::ws::{self, WsCommand, WsConfig, WsEvent, WsIncoming, WsOutgoing};

impl App {
    /// All live subscriptions: the active websocket session (if any) and the workspace file watch.
    pub fn subscription(&self) -> Subscription<Message> {
        let ws_sub = match &self.ws {
            Some(rt) if rt.active => ws::connect(&rt.config).map(Message::Ws),
            _ => Subscription::none(),
        };
        let watch_sub =
            crate::watch::watch(&self.workspace_dir).map(|()| Message::WorkspaceChanged);
        Subscription::batch([ws_sub, watch_sub])
    }

    /// Open a connection for the active websocket tab.
    pub(super) fn ws_connect(&mut self) {
        let Some(i) = self.active else {
            return;
        };
        let path = self.tabs[i].path.clone();
        let req = match find_node(&self.workspace.root, &path) {
            Some(Node::Ws(w)) => w.clone(),
            _ => return,
        };
        let auth = resolve_auth(&self.workspace.root, &path);
        let env = self
            .active_env
            .and_then(|idx| self.workspace.environments.get(idx));
        let ctx = runtime::VarContext::from_scopes(&self.workspace.globals, env);
        let config = build_config(&req, &auth, &ctx);
        self.ws = Some(WsRuntime::new(path, config));
    }

    pub(super) fn ws_disconnect(&mut self) {
        if let Some(rt) = &mut self.ws {
            if let Some(out) = &mut rt.out {
                let _ = out.try_send(WsCommand::Close);
            }
            rt.active = false;
            rt.connected = false;
            rt.log(WsDir::Info, "Disconnected.");
        }
    }

    pub(super) fn ws_event(&mut self, event: WsEvent) {
        let Some(rt) = &mut self.ws else {
            return;
        };
        match event {
            WsEvent::Connected(sender) => {
                rt.out = Some(sender);
                rt.connected = true;
                rt.log(WsDir::Info, "Connected.");
            }
            WsEvent::Received(inc) => rt.log(WsDir::In, incoming_text(&inc)),
            WsEvent::Failed(e) => {
                rt.connected = false;
                rt.active = false;
                rt.error = Some(e.clone());
                rt.log(WsDir::Info, format!("Error: {e}"));
            }
            WsEvent::Closed => {
                rt.connected = false;
                rt.active = false;
                rt.log(WsDir::Info, "Closed.");
            }
        }
    }

    pub(super) fn ws_composer_changed(&mut self, text: String) {
        if let Some(rt) = &mut self.ws {
            rt.composer = text;
        }
    }

    pub(super) fn ws_kind_changed(&mut self, kind: WsKind) {
        if let Some(rt) = &mut self.ws {
            rt.kind = kind;
        }
    }

    pub(super) fn ws_send(&mut self) {
        if let Some(rt) = &mut self.ws
            && !rt.composer.is_empty()
        {
            let text = std::mem::take(&mut rt.composer);
            send_line(rt, text);
        }
    }

    pub(super) fn ws_send_saved(&mut self, idx: usize) {
        let path = match &self.ws {
            Some(rt) => rt.path.clone(),
            None => return,
        };
        let message = match find_node(&self.workspace.root, &path) {
            Some(Node::Ws(w)) => w.messages.get(idx).cloned(),
            _ => None,
        };
        if let (Some(message), Some(rt)) = (message, &mut self.ws) {
            rt.kind = message.kind;
            send_line(rt, message.content);
        }
    }
}

fn send_line(rt: &mut WsRuntime, text: String) {
    let outgoing = match rt.kind {
        WsKind::Binary => WsOutgoing::Binary(text.clone().into_bytes()),
        _ => WsOutgoing::Text(text.clone()),
    };
    let sent = match &mut rt.out {
        Some(out) => out.try_send(WsCommand::Send(outgoing)).is_ok(),
        None => {
            rt.log(WsDir::Info, "Not connected.");
            return;
        }
    };
    if sent {
        rt.log(WsDir::Out, text);
    } else {
        rt.log(WsDir::Info, "Send failed.");
    }
}

fn incoming_text(inc: &WsIncoming) -> String {
    match inc {
        WsIncoming::Text(t) => t.clone(),
        WsIncoming::Binary(n) => format!("<binary {n} bytes>"),
        WsIncoming::Ping => "<ping>".to_string(),
        WsIncoming::Pong => "<pong>".to_string(),
        WsIncoming::Close => "<close>".to_string(),
    }
}

fn build_config(req: &WsRequest, auth: &Auth, ctx: &runtime::VarContext) -> WsConfig {
    let url = runtime::interpolate(&req.url, ctx);
    let mut headers: Vec<(String, String)> = req
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| {
            (
                runtime::interpolate(&h.key, ctx),
                runtime::interpolate(&h.value, ctx),
            )
        })
        .collect();
    match auth {
        Auth::Bearer { token } | Auth::OAuth2 { token } => headers.push((
            "Authorization".to_string(),
            format!("Bearer {}", runtime::interpolate(token, ctx)),
        )),
        Auth::ApiKey {
            key,
            value,
            add_to: ApiKeyLoc::Header,
        } => headers.push((
            runtime::interpolate(key, ctx),
            runtime::interpolate(value, ctx),
        )),
        _ => {}
    }
    let subprotocols = req
        .subprotocols
        .iter()
        .map(|s| runtime::interpolate(s, ctx))
        .collect();
    WsConfig {
        url,
        headers,
        subprotocols,
    }
}
