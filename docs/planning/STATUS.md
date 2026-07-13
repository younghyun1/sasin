# Status — P0 through P16 delivered

Snapshot of the execution against [03-roadmap.md](03-roadmap.md). Branch `feat/p0-stabilize`.

## Commit ledger

| Phase | Commit | Summary |
|-------|--------|---------|
| docs | `94d8279` | Execution plan |
| P0 stabilize | `018e8e4` | bug fixes (B1/B2/B3/B5), clippy clean |
| P1 storage | `5805c8a` | git-native TOML model + storage, round-trip tests |
| P2 GUI shell | `7b6105b` | collection tree + tabs (iced 0.14, workspace model) |
| chore | `8ca2428`,`7fec73f` | panic=abort build; removed dead code (config/RequestModel/orphan) |
| P3a exec | `7d9e5df`,`9a5d514` | reqwest exec from Settings + auth + all body modes |
| P3b editor | `c3538a6` | Params/Headers/Auth/Body/Settings panels |
| P4 env/vars | `25d2031` | `{{var}}` + dynamic interpolation, env selector + editor |
| P5 curl | `d0026ea` | curl import (paste) + export (copy) |
| P6 scripting | `a1a7aa9` | rquickjs `pm.*` pre-request + test scripts |
| P7 websocket | `52f08d3` | tokio-tungstenite console via iced Subscription |
| P8 S1 chore | `eb5ef3c` | drop dead_code allows; storage recursion guard |
| P8 S2 runner | `b801ca1` | runner core (flatten/plan/data/report) + tests |
| P8 S3 runner GUI | `2066c00` | run-folder panel, data file, pass/fail summary |
| P8 S4 response | `cf2b327` | response tabs Body/Headers/Cookies/Preview + search + save-example |
| P8 S5 highlight | `2a1c1e0` | syntect highlighting in body/script editors |
| P8 S6 history | `83d455c` | persisted request history + sidebar list |
| P8 S7 cookies | `1995510` | shared session cookie jar + cookie manager |
| P8 S8 watch | `4d33d64` | notify file-watch reload on external change |
| P8 S9 websocket | `88679b4` | WS auto-reconnect + concurrent sessions |
| P8 review | `e593fd5` | adversarial-review fixes (see below) |
| P9 stabilize | `b7af888` | dep upgrade: reqwest 0.13 (+`form`/`query` features), toml 1.x, rquickjs 0.12, tungstenite 0.30, minors; purged resurrected pre-refactor orphans from the working tree |
| P10 W1+W2 | `206af63` | custom dark/light theme (Postman-orange accent, dark default) + persisted TOML UI prefs (window, splits, theme) |
| P10 W3-W5 | `d29a157` | embedded Inter / JetBrains Mono / Lucide fonts; mono code surfaces; highlighter follows theme |
| P10 W6+W7 | `b8268a6` | split widget restructure + themed divider (hover/drag accent); component polish: underline tabs, status pill/chips, tree icons + aligned badges, KV headers, status bar |
| P11 ergonomics | `d13ec19` | global shortcuts (Ctrl+Enter/S/W/T/F, Esc) + tree rename/duplicate/new-folder/new-request-in/move up-down; messages module split |
| P12 response body | `696f811` | ResponseBody Text/Binary + 10 MiB capped capture, hex + inline image preview, save/copy body |
| P13 search | `ec4d4e7` | sidebar flat search over the live index (name/path/method/url); history filter/show-more/clear |
| P14 postman | `da108d9` | Postman Collection v2.1 import (folders/bodies/auth/scripts/variables + warnings); folder variables now interpolate (globals < folders < env). **Overrides locked decision D3** (curl-only) per the Postman-parity goal; import only, no export |
| P15 cookies | `35b3b38` | cookie jar persisted to the cache dir (cookies.json), restored on boot, flushed on close; per-cookie delete + add row |
| P16 snippets | `2e3142f` | snippet copy as curl/HTTPie/JS fetch/Python requests; request Docs panel (description) |

Each phase: `cargo clippy --all-targets -D warnings` clean (default + `--no-default-features`),
tests green, sillok-logged. P0/P1/P2/P3a reviewed inline; P3b–P7 and P8 each reviewed in an
adversarial review→verify workflow pass.

## What works (Postman parity)

- **Storage**: human-readable TOML dir-tree (workspace=dir, collection=folder, one file per
  request/env), byte-stable round-trip, legacy `.sasin` migration, gitignored binary cache.
- **Requests**: method/url, query-param + header KV tables (enabled toggles), auth
  (none/inherit/basic/bearer/api-key/oauth2) with folder inheritance, all body modes
  (raw+language, urlencoded, form-data text+file, binary, GraphQL), per-request settings
  (timeout/redirects/TLS/proxy/cookies). Send with hard cancel + per-tab staleness.
- **Environments**: per-env variable tables, active-env selector, `{{var}}` + dynamic
  `{{$timestamp/$isoTimestamp/$randomUUID/$randomInt}}` resolution across the whole request.
- **Scripting** (feature `scripting`, default on): `pm.environment/variables/globals`,
  `pm.response` (code/json/text/headers/responseTime + `to.have.status/header`), `pm.test`,
  `pm.expect` (equal/eql/include/be.above/below), `console.*`. Pre-request feeds variables;
  test results + console shown in the response pane.
- **curl**: paste-to-import; copy-as-curl to clipboard.
- **WebSocket**: connect/disconnect, send text/binary, transcript, saved messages, ping/pong,
  connect headers + auth + subprotocols (interpolated).
- **Shell**: collection tree (expand/open/delete), multi-tab editing, resizable panes.

## P8 delivered (runner + polish)

- **Collection runner**: flatten a folder/workspace into an ordered request list, run sequentially
  with iteration count + CSV/JSON data file (per-iteration variable overrides), live pass/fail +
  assertion summary. pm.* scripts run on the UI thread; the GUI steps the plan via the send path.
- **Response panel tabs**: Body (pretty/raw + line search), Headers, Cookies (parsed Set-Cookie),
  Preview (HTML source / image note); save-as-example writes a `.http` dump to `<req>.examples/`.
- **Syntax highlighting**: iced `highlighter` (syntect) on raw-body, GraphQL, and script editors.
- **Persisted history**: every send recorded (capped) + sidebar list; click re-creates the request.
- **Cookie manager**: session-wide shared jar (cookies persist across sends) + view/clear UI.
- **File-watch reload**: `notify` subscription reloads on external change (git pull) when the
  on-disk tree differs from memory — own saves never loop.
- **WebSocket**: auto-reconnect with exponential backoff + multiple concurrent sessions.
- **Robustness**: storage recursion-depth guard (symlink-cycle safe); dropped the temporary
  `#![allow(dead_code)]` on `model`/`storage` (compiles clean without it).

## Remaining / out of scope

- Socket.IO and protobuf WS framing (out of scope — raw ws/wss only).
- HTML *rendering* in Preview (shows source; no embedded browser — a webview dependency
  would dwarf the app). Images preview inline since P12.
- OAuth2 grant flows (token paste only); Postman export (import only, per user decision);
  OpenAPI/HAR import; drag-and-drop reorder (up/down buttons instead); `pm.sendRequest`.
- Multi-process temp-file naming on save — low risk for a local single-process tool.
- WebSocket `verify_tls=false` is not honoured (WS TLS is always verified); disabling it would
  need a hand-rolled rustls dangerous verifier + crypto provider. The WS connect timeout *is*
  applied. The HTTP path honours `verify_tls`.
- Behavioral note (P9): reqwest 0.13 validates HTTP TLS via `rustls-platform-verifier` (the OS
  trust store) instead of bundled webpki roots — corporate/self-signed CAs installed system-wide
  now work without `verify_tls=false`.

## P8 adversarial review

A review→verify workflow (5 dimensions, findings each independently verified) confirmed 5 issues,
all fixed before close:
- **high** `reload_workspace` left non-dirty tab buffers stale → the next send/save flushed old
  text back over an externally-pulled change (silent data loss). Now non-dirty tabs are reseeded
  from the reloaded node; dirty tabs are kept and flagged.
- **med** runner data-file reload error left stale rows active → now cleared.
- **med** WS connect could hang forever (no timeout) → `connect_timeout_ms` now bounds it.
- **low** runner dropped a pre-request script error when the send also failed → now combined.
- **low** history re-open used a positional index that a cap-drain could shift → now passes the
  record by value.

## Caveat

The GUI and WebSocket paths could not be exercised at runtime in the build environment
(no display / no WS server); they are verified by compilation, unit tests on the pure layers,
and adversarial review against the iced / reqwest / rquickjs / tungstenite sources.
