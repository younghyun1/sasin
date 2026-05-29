# Status — P0 through P7 delivered

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

Each phase: `cargo clippy --all-targets -D warnings` clean, tests green, sillok-logged.
P0/P1/P2/P3a were adversarially reviewed inline; P3b–P7 reviewed in a consolidated pass.

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

## Deferred to P8 (polish / runner)

- Collection runner (sequential, iterations + data file, aggregate results).
- Response panel tabs: Cookies, Preview (HTML/image), search, save-as-example.
- Syntax highlighting in body/response/script editors (iced `highlighter` feature).
- Persisted request history UI; cookie-manager UI; file-watch reload on git pull.
- WebSocket: auto-reconnect, multiple concurrent sessions, Socket.IO (out of scope).
- Robustness (from P1/P3a reviews): symlink-cycle / recursion-depth guards, multi-process
  temp-file naming — low risk for a local single-process tool.
- Drop the temporary `#![allow(dead_code)]` on `model`/`storage` (most is now used).

## Caveat

The GUI and WebSocket paths could not be exercised at runtime in the build environment
(no display / no WS server); they are verified by compilation, unit tests on the pure layers,
and adversarial review against the iced / reqwest / rquickjs / tungstenite sources.
