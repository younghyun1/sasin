# 00 — Current state audit

Snapshot of the compiled tree at plan start. Commit `e4ac709` ("feature: collections").

## What compiles and works

| Area | State |
|------|-------|
| HTTP send | `reqwest`, methods GET/POST/PUT/DELETE/PATCH/HEAD/OPTIONS, headers, raw body, timeout, UA. Returns status/headers/body/duration. |
| Cancel | Hard cancel via `tokio` `AbortHandle` + generation counter to drop stale results. |
| GUI shell | iced 0.14: method picker, URL input, headers (raw multiline `text_input`), body (`text_editor`), Send/Cancel. |
| Response view | Status/duration/size, headers toggle, body, pretty-JSON toggle (**toggle only — not implemented**). |
| Collections | Flat collections → requests. New/Save/Delete/Rename. "Immediate mutation" of selected request. |
| Persistence | Single binary `.sasin` file (`bitcode` + `zstd`), atomic write, v1→v2 migration, debounced autosave. |
| Config | Window/layout/last-dataset-path in `config.sasin` (bitcode). Cross-platform `app_state_dir`. |
| Layout | Custom resizable `Split` widget (advanced iced API), 2 axes, persisted px. |
| History | In-memory only (**not persisted**). |

## Module map

```
src/
  main.rs            iced::run entrypoint, mimalloc, logger
  models/            RequestModel, HttpMethod, ResponseModel  (runtime HTTP types)
  http/client.rs     send(config, req) -> ResponseModel
  persist/
    dataset.rs       Dataset{collections:[Collection{requests:[Request]}]}, binary codec
    config.rs        AppConfig + binary codec + load_startup_dataset
    paths.rs         XDG/APPDATA/macOS path resolution
    workspace.rs     ORPHAN sketch — not in mod.rs, does not compile
  gui/
    app.rs           App (the big update/view, 909 LOC)
    messages.rs      Message enum
    state/dataset_sync.rs  editor<->dataset glue, header parse/format
    components/      collection_view, request_editor, response_view, history_list, section, split
```

## Bugs to fix (carry into Phase 0)

| # | Location | Problem | Fix |
|---|----------|---------|-----|
| B1 | `components/request_editor.rs:129` | Request-name input emits `RenameRequestPressed(0, s)` — id `0` never matches, so renaming via the name field is a no-op; `RequestNameChanged` is never emitted from the UI. | Emit `RequestNameChanged`; let `app` route to rename-selected. |
| B2 | `state/dataset_sync.rs:162` | Header parse error uses `"{{line}}"` (escaped braces) → prints literal `{line}` instead of the line text. | Interpolate the offending line. |
| B3 | `gui/app.rs:750` | `ResponseView::body_text(None)` always → pretty-JSON / formatting never applied despite the toggle. | Compute formatted body in `app`/response layer and pass it in. |
| B4 | history | History lost on exit. | Persist to cache (`.sasin-cache/history`). |
| B5 | `main.rs:1` | `#![feature(const_type_name)]` declared, unused (warning). Also pins nightly. | Remove feature; confirm stable toolchain (or document nightly need). |

## Gaps vs Postman (HTTP + WS)

Missing entirely: query-param table, body modes (form-data/urlencoded/binary/graphql), rich auth,
environments + `{{var}}` substitution, scripting/tests, cookie jar, per-request settings
(redirects/TLS/proxy), nested folders, tabs (multiple open requests), response preview/search/save,
curl import/export, WebSocket, collection runner, persisted history.

Full matrix: [04-parity-checklist.md](04-parity-checklist.md).

## Disposition of existing code

- **Keep + extend**: `models` (runtime HTTP types), `http/client.rs` (becomes `http/exec.rs` core),
  `gui/components/split`, `section`, `paths.rs`, the bitcode+zstd codec (repurposed for the cache).
- **Replace as source of truth**: `persist/dataset.rs` binary file → TOML tree (Phase 1). The binary
  codec survives as the gitignored `.sasin-cache/` index. One-time migration imports old `.sasin`.
- **Supersede**: `persist/workspace.rs` sketch → real model in `src/model/` + `src/storage/`.
