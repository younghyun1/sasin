# 03 — Roadmap

Phases run in order; **each ends in a compiling, usable app**. Effort: S<½d, M~1–2d, L~3–5d, XL>1wk.

## Dependency graph

```
P0 fixes ─► P1 storage ─► P2 GUI shell ─► P3 HTTP surface ─┬─► P4 environments ─► P6 scripting
                                                            ├─► P5 curl
                                                            └─► P7 websocket ─► P8 runner+polish
```

P4/P5/P7 are independent after P3 and may interleave. P6 (scripting) needs P4 (vars) for
`pm.environment`. P8 needs the HTTP+WS exec from P3/P7.

---

## Phase 0 — Stabilize & prep · **S**

**Goal:** fix known bugs, pin toolchain, add foundational deps. No new features.

- Fix B1–B5 ([00-current-state.md](00-current-state.md)).
- Add deps: `serde` (derive), `toml`; expand `reqwest` features
  (`json`, `multipart`, `cookies`, `rustls-tls`, `gzip`, `brotli`, `deflate`, `stream`).
- Confirm/declare toolchain (remove unused nightly `feature`, or document why nightly is required).
- Introduce `cargo clippy -- -D warnings` to the dev loop.

**DoD:** `cargo clippy` clean; existing flows work; pretty-JSON + rename + persisted history work.

---

## Phase 1 — Git-native storage core · **L**

**Goal:** TOML dir-tree as source of truth; binary cache; migration. (Headless-testable, no GUI change yet.)

- `model/` canonical types (workspace/tree/request/ws/body/auth/env/scripts/method).
- `storage/`: `layout` (path↔id, slugging), `load`, `save` (atomic, dirty-only, stable bytes),
  `cache` (index/session/history/cookies), `migrate` (old `.sasin` → tree).
- Auto-write `.gitignore` (`.sasin-cache/`) on workspace create.
- Tests: invariants 1–5 from [02-storage-format.md](02-storage-format.md) (+ `fuzztest` on the TOML parser path).

**DoD:** create/open a workspace dir; edit a request in code → one TOML file changes; round-trip +
migration tests green.

**Risk:** `toml` serialization stable-byte fidelity (mitigate: golden-file tests; pin formatting).

---

## Phase 2 — GUI shell: tree + tabs + new model · **L**

**Goal:** rebuild the shell around the workspace model; open requests in tabs; save to TOML.

- `components/tree.rs` (recursive, expand/collapse, select, context menu: new/rename/delete/duplicate).
- `components/tabs.rs`; `state/` session (open tabs, active env placeholder, selection, dirty).
- Port request editor to the new model (panels can stay basic this phase); Send via current client.
- Persist/restore session via cache.

**DoD:** navigate tree, open multiple requests in tabs, edit + save to TOML, send, restore tabs on relaunch.

**Risk:** iced 0.14 custom widgets (tree/tabs) — reuse `Split` patterns; budget for advanced-API friction.

---

## Phase 3 — Full HTTP request surface · **XL**

**Goal:** Postman-grade request builder + response panel (minus env/scripting).

- `components/kv_table.rs` (params/headers) + URL⇄params sync.
- Body modes: raw(+language highlight), urlencoded, formdata (text/file), binary, graphql.
- `model/auth` + `runtime/auth_apply` + inheritance (none/inherit/basic/bearer/apikey/oauth2-token).
- `http/client.rs` rebuild: per-request `reqwest::Client` from `Settings`
  (redirects, TLS verify, proxy, shared cookie jar); `http/exec.rs` pipeline; `http/response.rs`.
- `response_panel/`: Pretty | Raw | Preview (HTML/image) | Headers | Cookies; search; save-as-example; copy.
- `gui/theme.rs` central styling; `highlighter` feature for editors.

**DoD:** build + send any HTTP request with all body modes + auth; rich response panel; save examples.

**Risk:** formdata/file streaming + multipart; response preview rendering scope (keep HTML/image only).

---

## Phase 4 — Environments & variables · **M**

**Goal:** `{{var}}` resolution end-to-end.

- `model/environment` + `components/env_manager.rs` + active-env selector in toolbar.
- `runtime/vars.rs`: scope chain (request/folder→env→globals), dynamic `{{$…}}`, missing-var policy.
- Apply interpolation in `http/exec` to a resolved clone; secret masking in UI.

**DoD:** switch env; vars resolve across url/params/headers/body/auth; dynamic vars work; secrets masked.

---

## Phase 5 — curl import/export · **M**

**Goal:** paste-curl→request; request→curl.

- `interop/curl_import.rs` (tokenize a curl command: `-X`,`-H`,`-d`/`--data*`,`-F`,`--url`,`-u`,
  `--compressed`, cookies, method inference) → `HttpRequest`.
- `interop/curl_export.rs` → copyable curl string honoring current body mode/auth/headers.
- Toolbar actions + "import from curl" paste box. `fuzztest` the parser.

**DoD:** round-trip common curl commands; export reproduces an equivalent request.

---

## Phase 6 — Scripting (pm.* JS sandbox) · **XL** · feature `scripting`

**Goal:** pre-request + test scripts with a Postman-like API.

- `scripting/engine.rs` (`rquickjs` context per run; interrupt/time + memory budget; no fs/net).
- `scripting/prelude.js`: `pm.environment`/`globals`/`variables` (get/set/unset), `pm.request`,
  `pm.response` (code/headers/json()/text()/responseTime), `pm.test`, `pm.expect` (chai-subset),
  `console.*` (captured).
- `scripting/pm_api.rs`: Rust↔JS bridge; feed script-set vars back into the interpolation scope.
- Wire into `http/exec` (pre before send, test after); `response_panel` **Tests** tab + console output.

**DoD:** Postman-style pre-request mutates vars; tests produce pass/fail rows; console captured.
Building without the feature stores scripts but skips execution (UI hint).

**Risk (highest):** rquickjs native build on the target toolchain; pm-API surface creep
(scope tightly to the common subset); sandbox time/memory limits. Mitigate with the feature gate.

---

## Phase 7 — WebSocket · **L** · feature `ws`

**Goal:** raw WS testing.

- `ws/exec.rs` (`tokio-tungstenite` + rustls): connect with interpolated url/headers/subprotocols/auth.
- iced `Subscription` pumping frames → `Message`; outbound queue; ping/pong; auto-reconnect+backoff.
- `components/ws_console.rs`: connect bar, composer (text/binary/json), transcript (dir-colored, ts),
  saved messages list. WS model already persisted (Phase 1).

**DoD:** connect, send/recv text+binary, live transcript, save/load WS requests, reconnect.

---

## Phase 8 — Collection runner & polish · **L**

**Goal:** run folders/collections; finish the long tail.

- Sequential runner over a folder/collection; iterations + data file (CSV/JSON) → per-iteration vars;
  aggregate test results.
- Cookie manager UI; persisted history UI; `notify` file-watch reload on git pull/branch switch;
  keyboard shortcuts; theming pass; perf check on large workspaces (index/cache).

**DoD:** run a collection with a data file; pass/fail summary; external git changes reload cleanly.

---

## New dependencies (by phase)

| Crate | Phase | Use |
|-------|-------|-----|
| `serde` (derive), `toml` | P0/P1 | TOML model (de)serialization |
| `reqwest` features `json,multipart,cookies,rustls-tls,gzip,brotli,deflate,stream` | P0/P3 | full HTTP |
| `iced` feature `highlighter` (syntect) | P3 | body/response/script highlighting |
| `rquickjs` | P6 | JS sandbox (feature `scripting`) |
| `tokio-tungstenite`, `rustls`/`tokio-rustls` | P7 | WebSocket (feature `ws`) |
| `notify` | P8 | filesystem watch |
| `csv` | P8 | runner data files |
| `fuzztest` | P1/P5/P6 | fuzz parsers (toml/curl/interpolation) |

Pin exact versions at implementation time (latest stable). `scripting` + `ws` are cargo features,
default-on for releases, off for fast core builds/tests and to isolate native deps.

## Definition of done (every phase)

`cargo fmt` + `cargo clippy -D warnings` clean · unit tests for new logic · no `unwrap`/`expect` ·
files <300 LOC · `mod.rs` = decls only · rustdoc on new modules/fns · sillok note + task update.
