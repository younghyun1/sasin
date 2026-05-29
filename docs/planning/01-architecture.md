# 01 — Target architecture

## Layering

```
                ┌─────────────────────────────────────────────┐
   GUI (iced)   │ tabs · collection tree · request/ws editors  │
                │ response panel · env mgr · console           │
                └───────────────┬─────────────────────────────┘
                                │ Message / Task
                ┌───────────────▼─────────────────────────────┐
   Runtime      │ exec (http/ws) · interpolation · scripting   │
                │ auth · cookie jar · curl import/export        │
                └───────────────┬─────────────────────────────┘
                                │ load / save (typed model)
                ┌───────────────▼─────────────────────────────┐
   Storage      │ TOML dir-tree (source of truth)             │
                │ + .sasin-cache binary index (derived)        │
                └─────────────────────────────────────────────┘
```

Strict dependency direction: GUI → Runtime → Storage → Model. Model depends on nothing.
No `reqwest`/`iced` types leak into Model or Storage.

## Crate / module layout (target)

```
src/
  main.rs
  model/                  # pure data, serde-(de)serializable, no I/O
    mod.rs
    workspace.rs          Workspace, WorkspaceDefaults
    tree.rs               Node = Folder | Http | Ws; folder metadata + ordering
    request.rs            HttpRequest, Param, Header, Settings
    websocket.rs          WsRequest, WsMessageTemplate
    body.rs               Body { mode, ... } enum
    auth.rs               Auth enum (None/Inherit/Basic/Bearer/ApiKey/OAuth2)
    environment.rs        Environment, Variable
    scripts.rs            Scripts { pre_request, test }
    http_method.rs        HttpMethod (moved from models/)
  storage/                # TOML tree <-> model, atomic per-file I/O
    mod.rs
    layout.rs             path<->id rules, slugging, folder.toml ordering
    load.rs               dir -> Workspace
    save.rs               model -> dir (atomic, only-dirty)
    cache.rs              .sasin-cache: index/session/history/cookies (bitcode+zstd)
    migrate.rs            old .sasin binary -> TOML tree (one-time)
    paths.rs              (from persist/paths.rs)
  runtime/
    mod.rs
    vars.rs               {{var}} + {{$dynamic}} interpolation, scope chain
    auth_apply.rs         Auth -> headers/url (with inheritance)
    cookies.rs            cookie jar (shared per workspace)
  http/
    mod.rs
    client.rs             reqwest client builder from Settings (redirects/tls/proxy/cookies)
    exec.rs               run HttpRequest -> Execution (pre-script, send, test-script)
    response.rs           ResponseModel (status/headers/body/timing/size)
  ws/
    mod.rs
    exec.rs               tokio-tungstenite connect/send/recv loop
    session.rs            live connection state, transcript
  scripting/              # feature = "scripting"
    mod.rs
    engine.rs             rquickjs runtime, interrupt/time limit
    pm_api.rs             Rust<->JS bridge (env/vars/request/response/test)
    prelude.js            pm.* + expect/chai-subset implemented in JS
  interop/
    mod.rs
    curl_import.rs        curl string -> HttpRequest
    curl_export.rs        HttpRequest -> curl string
  gui/
    app.rs                top-level App (thin: routes to panels/tabs)
    messages.rs
    state/                session, selection, tab state, dirty tracking
    components/
      tree.rs             recursive collection tree (custom widget)
      tabs.rs             tab bar
      kv_table.rs         key/value table (params, headers, urlencoded, formdata, vars)
      request_editor/     top bar + Params|Headers|Auth|Body|Scripts|Settings sub-tabs
      response_panel/     Pretty|Raw|Preview|Headers|Cookies|Tests sub-tabs
      ws_console.rs       connect bar + composer + transcript
      env_manager.rs
      split.rs, section.rs   (kept)
```

`mod.rs` files hold only `pub mod` / `pub use` (AGENTS.md). Split any file >300 LOC.

## Domain model (canonical)

Identity is the **relative path** within the workspace (D5). No UUIDs in files. The in-memory
node may carry a transient `id` derived from path for GUI bookkeeping only.

```rust
struct Workspace { name: String, defaults: WorkspaceDefaults, root: Vec<Node>, environments: Vec<Environment>, globals: Vec<Variable> }

enum Node { Folder(Folder), Http(HttpRequest), Ws(WsRequest) }

struct Folder { slug: String, name: String, description: Option<String>,
                auth: Auth, variables: Vec<Variable>, scripts: Scripts,
                order: Vec<String>, children: Vec<Node> }

struct HttpRequest { slug: String, name: String, description: Option<String>,
                     method: HttpMethod, url: String,
                     params: Vec<Param>, headers: Vec<Header>,
                     auth: Auth, body: Body, settings: Settings, scripts: Scripts,
                     examples: Vec<ExampleRef> }

struct WsRequest { slug, name, description, url, headers, auth, subprotocols: Vec<String>,
                   settings: WsSettings, messages: Vec<WsMessageTemplate> }

enum Body { None, Raw{language: RawLang, text: String}, UrlEncoded(Vec<KvEntry>),
            FormData(Vec<FormPart>), Binary{file: String}, GraphQl{query, variables} }

enum Auth { None, Inherit, Basic{user,pass}, Bearer{token},
            ApiKey{key,value,add_to: ApiKeyLoc}, OAuth2{ /* token + grant cfg */ } }

struct Scripts { pre_request: Option<String>, test: Option<String> }   // reserved even pre-Phase-6
struct Variable { key: String, value: String, enabled: bool, secret: bool, description: Option<String> }
```

Auth/vars/scripts resolve through the **folder chain → environment → globals** (request overrides
folder overrides env overrides globals). `Inherit` walks parents.

## Runtime engines

**Interpolation (`runtime/vars.rs`).** Resolve `{{name}}` and dynamic `{{$timestamp}}`,
`{{$isoTimestamp}}`, `{{$randomUUID}}`, `{{$randomInt}}`, `{{$guid}}`. Build a `VarContext`
from the scope chain + script-set locals. Applied to a **resolved clone** at send time; stored
request is never mutated. Missing var → configurable (leave literal vs error; default: leave + warn).

**HTTP exec (`http/exec.rs`).** Pipeline:
`resolve scope → run pre_request script (may set vars) → interpolate → build reqwest client from
Settings (redirects, TLS verify, proxy, shared cookie jar) → apply Auth → encode Body → send →
capture Response → run test script → collect TestResults`. Returns `Execution { response, tests, console, timing }`.

**WS exec (`ws/exec.rs`).** `tokio-tungstenite` connect with interpolated url/headers/subprotocols/TLS;
spawn read loop pushing frames to the GUI via an iced `Subscription`/channel; outbound send queue;
ping/pong; optional auto-reconnect with backoff. Transcript = `Vec<{dir, kind, bytes, at}>`.

**Scripting (`scripting/`, feature-gated).** `rquickjs` context per execution (fresh sandbox, no
fs/net). `prelude.js` defines `pm` (`environment`/`globals`/`variables` get/set, `request`,
`response`, `test`, `expect` via a chai-subset) and `console`. Rust bridges read/write var maps and
expose response data. Interrupt handler enforces a wall-clock/instruction budget. Build without the
feature → scripts are stored but not executed (graceful no-op + UI hint).

## GUI architecture

- **App** stays thin: owns `Workspace`, `Session` (open tabs, active env, selection, splits), and
  in-flight exec handles. Delegates rendering to components; mutations go through Storage save.
- **Tabs**: `Vec<Tab>` (`Http | Ws | Env`), each tab holds its own editor draft + dirty flag.
  Persisted in cache; restored on launch.
- **Tree**: recursive widget over `Node`; expand/collapse, select, context menu
  (new/rename/delete/duplicate/move). Drag-reorder writes `folder.toml` `order`.
- **KV table**: shared component for params/headers/urlencoded/formdata/vars (enabled checkbox,
  add/remove row, bulk edit).
- **Editors**: `iced` `text_editor` with the `highlighter` feature (syntect) for body + response +
  scripts. Centralize palette/spacing constants in one `gui/theme.rs` (AGENTS.md "centralize styling").
- **Async**: send/ws via `Task`/`Subscription`; keep the existing generation-counter + `AbortHandle`
  staleness/cancel pattern.

## Cross-cutting conventions

No `unwrap`/`expect`; `match` on `Option`/`Result`; structured `tracing` fields; `spawn_blocking`
for blocking work; drop large buffers promptly; unit tests per module (`fuzztest` for parsers:
header/curl/interpolation). Feature flags: `scripting` (rquickjs), `ws` (tokio-tungstenite) — both
default-on for releases, toggleable to keep build/test fast and isolate native deps.
