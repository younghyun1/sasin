//! Shared fixtures and helpers for storage tests.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{
    ApiKeyLoc, Auth, Body, Environment, Folder, FormKind, FormPart, HttpRequest, KvEntry, Node,
    RawLang, Scripts, Settings, Variable, Workspace, WorkspaceDefaults, WsKind, WsMessageTemplate,
    WsRequest, WsSettings,
};

/// A unique temp directory path for a test (not created — callers save into it).
pub(super) fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "sasin-storage-{tag}-{}-{nanos}",
        std::process::id()
    ))
}

/// A workspace exercising every body mode, auth variant, websocket, environments, and a
/// deliberately non-lexical child order.
pub(super) fn sample() -> Workspace {
    let mut ws = Workspace::default_with_name("Demo");
    ws.defaults = WorkspaceDefaults::default();
    ws.globals = vec![Variable::new("global_key", "gval")];
    ws.environments = vec![Environment {
        slug: "dev".to_string(),
        name: "dev".to_string(),
        variables: vec![
            Variable::new("base_url", "https://dev"),
            Variable {
                key: "token".to_string(),
                value: String::new(),
                enabled: true,
                secret: true,
                description: Some("secret".to_string()),
            },
        ],
    }];

    let login = HttpRequest {
        slug: "login".to_string(),
        name: "Login".to_string(),
        method: "POST".to_string(),
        url: "{{base_url}}/login".to_string(),
        params: vec![
            KvEntry::new("verbose", "1"),
            KvEntry {
                key: "off".to_string(),
                value: "x".to_string(),
                enabled: false,
            },
        ],
        headers: vec![KvEntry::new("Accept", "application/json")],
        auth: Auth::Basic {
            user: "u".to_string(),
            pass: "p".to_string(),
        },
        body: Body::Raw {
            language: RawLang::Json,
            text: "{\n  \"a\": 1\n}".to_string(),
        },
        settings: Settings {
            timeout_ms: 1000,
            follow_redirects: false,
            verify_tls: false,
            use_cookie_jar: false,
            proxy: Some("http://proxy".to_string()),
        },
        scripts: Scripts {
            pre_request: "pm.environment.set('x', 1)".to_string(),
            test: "pm.test('ok', () => {})".to_string(),
        },
        ..HttpRequest::default()
    };

    let upload = HttpRequest {
        body: Body::FormData {
            parts: vec![
                FormPart {
                    key: "f".to_string(),
                    kind: FormKind::File,
                    value: String::new(),
                    src: "a.bin".to_string(),
                    enabled: true,
                },
                FormPart {
                    key: "t".to_string(),
                    kind: FormKind::Text,
                    value: "v".to_string(),
                    src: String::new(),
                    enabled: true,
                },
            ],
        },
        auth: Auth::Bearer {
            token: "{{token}}".to_string(),
        },
        ..HttpRequest::new("upload", "Upload", "POST", "https://x/upload")
    };

    let form = HttpRequest {
        body: Body::UrlEncoded {
            fields: vec![KvEntry::new("a", "b")],
        },
        auth: Auth::ApiKey {
            key: "X-Key".to_string(),
            value: "v".to_string(),
            add_to: ApiKeyLoc::Header,
        },
        ..HttpRequest::new("form", "Form", "POST", "https://x/form")
    };

    let bin = HttpRequest {
        body: Body::Binary {
            file: "payload.bin".to_string(),
        },
        auth: Auth::None,
        ..HttpRequest::new("raw-bin", "Bin", "PUT", "https://x/bin")
    };

    let gql = HttpRequest {
        body: Body::GraphQl {
            query: "{ me { id } }".to_string(),
            variables: "{}".to_string(),
        },
        ..HttpRequest::new("gql", "GraphQL", "POST", "https://x/graphql")
    };

    let nested = Folder {
        children: vec![Node::Http(HttpRequest::new(
            "ban",
            "Ban",
            "DELETE",
            "https://x/ban",
        ))],
        ..Folder::new("admin", "Admin")
    };

    let auth_folder = Folder {
        auth: Auth::Bearer {
            token: "{{token}}".to_string(),
        },
        variables: vec![Variable::new("scope", "admin")],
        scripts: Scripts {
            pre_request: String::new(),
            test: "pm.test('t', () => {})".to_string(),
        },
        children: vec![Node::Http(login), Node::Folder(nested)],
        ..Folder::new("auth", "Auth")
    };

    // zebra before alpha: non-lexical, to verify order fidelity.
    let api_folder = Folder {
        children: vec![
            Node::Http(upload),
            Node::Http(form),
            Node::Http(bin),
            Node::Http(gql),
            Node::Http(HttpRequest::new("zebra", "Zebra", "GET", "https://x/z")),
            Node::Http(HttpRequest::new("alpha", "Alpha", "GET", "https://x/a")),
        ],
        ..Folder::new("api", "API")
    };

    let chat = WsRequest {
        slug: "chat".to_string(),
        name: "Chat".to_string(),
        url: "wss://x/chat".to_string(),
        subprotocols: vec!["json".to_string()],
        headers: vec![KvEntry::new("Origin", "https://x")],
        auth: Auth::Bearer {
            token: "{{token}}".to_string(),
        },
        settings: WsSettings {
            connect_timeout_ms: 1234,
            auto_reconnect: true,
            verify_tls: false,
        },
        messages: vec![WsMessageTemplate {
            name: "ping".to_string(),
            kind: WsKind::Json,
            content: "{\"t\":1}".to_string(),
        }],
        ..WsRequest::default()
    };

    ws.root = vec![
        Node::Folder(auth_folder),
        Node::Folder(api_folder),
        Node::Ws(chat),
    ];
    ws
}

/// Recursively collect `*.toml` + `.gitignore` file contents, keyed by relative path.
pub(super) fn read_tree(dir: &Path, base: &Path, out: &mut BTreeMap<String, String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            read_tree(&path, base, out);
        } else if (name.ends_with(".toml") || name == ".gitignore")
            && let (Ok(rel), Ok(content)) = (path.strip_prefix(base), fs::read_to_string(&path))
        {
            out.insert(rel.to_string_lossy().to_string(), content);
        }
    }
}
