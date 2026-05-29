# 04 — Postman parity checklist

Scope = Postman's **HTTP** + **WebSocket** surface. Status: ✅ done · 🟡 partial · ⬜ missing.
"Phase" = where it lands ([03-roadmap.md](03-roadmap.md)).

## Requests — HTTP

| Feature | Status | Phase |
|---------|:--:|:--:|
| Methods GET/POST/PUT/DELETE/PATCH/HEAD/OPTIONS | ✅ | — |
| Arbitrary/custom method | ⬜ | P3 |
| URL bar + Send | ✅ | — |
| Query-param table (key/value/enabled), URL⇄params sync | ⬜ | P3 |
| Path variables (`:id`) | ⬜ | P4 |
| Header table (enabled, bulk edit, presets) | 🟡 raw text | P3 |
| Body: raw (json/text/xml/html/js) + highlight | 🟡 raw, no highlight | P3 |
| Body: x-www-form-urlencoded | ⬜ | P3 |
| Body: form-data (text + file parts) | ⬜ | P3 |
| Body: binary (file) | ⬜ | P3 |
| Body: GraphQL (query + variables) | ⬜ | P3 |
| Auth: none / inherit | ⬜ | P3 |
| Auth: Basic / Bearer / API-key | ⬜ | P3 |
| Auth: OAuth2 (token use; full grant flows later) | 🟡 token only | P3/P8 |
| Per-request settings: timeout, redirects, TLS verify, proxy | ⬜ | P3 |
| Send + hard cancel | ✅ | — |

## Responses

| Feature | Status | Phase |
|---------|:--:|:--:|
| Status / time / size | ✅ | — |
| Body Pretty / Raw | 🟡 toggle not wired | P0/P3 |
| Body Preview (HTML/image) | ⬜ | P3 |
| Syntax highlight | ⬜ | P3 |
| Response headers view | ✅ | — |
| Response cookies view | ⬜ | P3 |
| Search in response | ⬜ | P3 |
| Save response as example | ⬜ | P3 |
| Copy / save-to-file | ⬜ | P3 |

## Collections & organization

| Feature | Status | Phase |
|---------|:--:|:--:|
| Collections | ✅ flat | — |
| Nested folders | ⬜ | P1/P2 |
| Reorder (drag) | ⬜ | P2/P8 |
| Duplicate / move | ⬜ | P2 |
| Multiple open tabs | ⬜ | P2 |
| Folder/collection-level auth + variables | ⬜ | P3/P4 |
| Folder/collection-level scripts | ⬜ | P6 |
| Request examples (saved responses) | ⬜ | P3 |

## Environments & variables

| Feature | Status | Phase |
|---------|:--:|:--:|
| Environments + active selector | ⬜ | P4 |
| `{{var}}` substitution (url/headers/params/body/auth) | ⬜ | P4 |
| Globals | ⬜ | P4 |
| Dynamic vars `{{$timestamp}}`, `{{$randomUUID}}`, … | ⬜ | P4 |
| Secret masking | ⬜ | P4 |
| Variable scope chain (request→folder→env→global) | ⬜ | P4 |

## Scripting & tests

| Feature | Status | Phase |
|---------|:--:|:--:|
| Pre-request script (JS) | ⬜ | P6 |
| Test script (JS) + results panel | ⬜ | P6 |
| `pm.*` API (environment/variables/request/response/test/expect) | ⬜ | P6 |
| `console.*` capture | ⬜ | P6 |
| `pm.sendRequest` | ⬜ | later |

## WebSocket

| Feature | Status | Phase |
|---------|:--:|:--:|
| Connect / disconnect (ws/wss) | ⬜ | P7 |
| Connect headers + subprotocols + auth | ⬜ | P7 |
| Send text / binary / json | ⬜ | P7 |
| Live transcript (direction, timestamp) | ⬜ | P7 |
| Ping/pong | ⬜ | P7 |
| Auto-reconnect | ⬜ | P7 |
| Saved outbound messages | ⬜ | P7 |
| Socket.IO | ❌ out of scope (D4) | — |

## Import / interop

| Feature | Status | Phase |
|---------|:--:|:--:|
| curl import (paste) | ⬜ | P5 |
| curl export (copy) | ⬜ | P5 |
| Postman Collection v2.1 | ❌ out of scope (D3) | — |
| OpenAPI import | ❌ out of scope (D3) | — |
| Code generation | ❌ out of scope (D3) | — |

## Runner & misc

| Feature | Status | Phase |
|---------|:--:|:--:|
| Collection runner (sequential) | ⬜ | P8 |
| Iterations + data file (CSV/JSON) | ⬜ | P8 |
| Aggregate test results | ⬜ | P8 |
| Cookie manager UI | ⬜ | P8 |
| Request history (persisted + UI) | 🟡 in-memory | P0/P8 |
| File-watch reload (git pull/branch switch) | ⬜ | P8 |
| Resizable panes | ✅ | — |
| Cross-platform state dirs | ✅ | — |

## Storage (the differentiator)

| Feature | Status | Phase |
|---------|:--:|:--:|
| Human-readable, diffable files | ⬜ (binary today) | P1 |
| Git-mergeable (per-request files) | ⬜ | P1 |
| Workspace = directory | ⬜ | P1 |
| Binary cache (gitignored, rebuildable) | 🟡 (is current source of truth) | P1 |
| Migration from legacy `.sasin` | ⬜ | P1 |
