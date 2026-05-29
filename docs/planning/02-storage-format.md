# 02 — Storage format (git-native)

Source of truth = a **workspace directory** of human-readable TOML. Everything diffs and merges in git.
A derived **binary cache** (`.sasin-cache/`, gitignored) accelerates startup/search and holds
session/history/cookies. The cache is always rebuildable from the tree; deleting it loses nothing
git-tracked.

## Layout

```
my-workspace/
  sasin.toml                     # workspace manifest
  .gitignore                     # auto-written: ".sasin-cache/"
  environments/
    globals.toml                 # workspace-wide variables
    dev.toml
    prod.toml
  Auth/                          # collection = folder (dir name = slug)
    folder.toml                  # folder name, order, folder-level auth/vars/scripts
    login.req.toml               # HTTP request
    refresh.req.toml
  Users/
    folder.toml
    list.req.toml
    get-by-id.req.toml
    list.req.examples/           # saved responses for list.req (sidecar dir)
      200-ok.toml
    Admin/                       # nested folder
      folder.toml
      ban-user.req.toml
  chat.ws.toml                   # WebSocket request (top-level)
  .sasin-cache/                  # DERIVED, gitignored
    index.bc.zst
    session.bc.zst
    history.bc.zst
    cookies.bc.zst
    responses/<path-hash>.bc.zst
```

**Identity = relative path** (D5). `Users/get-by-id.req.toml` *is* the request's id. Rename/move =
`git mv`; references (active tab, history) store the relative path. Reordering is metadata
(`folder.toml`), not filename prefixes — keeps renames clean.

**Slug vs name.** The filename stem (slug) is filesystem-safe and stable; `name` is the display
string and may contain spaces/unicode. Creating "Get by ID" → slug `get-by-id`, `name = "Get by ID"`.

**File-type discriminator.** `*.req.toml` = HTTP, `*.ws.toml` = WebSocket, `folder.toml` = folder
metadata, `environments/*.toml` = environment. Every file carries an explicit `schema` key for
forward-compat and validation.

## File schemas

### `sasin.toml` (workspace manifest)

```toml
schema = "sasin/workspace@1"
name   = "My Workspace"

[defaults]            # inherited by all requests unless overridden
timeout_ms       = 30000
follow_redirects = true
verify_tls       = true
use_cookie_jar   = true

order = ["Auth", "Users", "chat.ws"]   # top-level ordering (slugs, no extension)
```

### `folder.toml`

```toml
schema      = "sasin/folder@1"
name        = "Users"
description = "User CRUD"
order       = ["list.req", "get-by-id.req", "Admin"]

[auth]                       # folder-level, inherited by children that use Inherit
type  = "bearer"
token = "{{access_token}}"

[[variable]]                 # folder-scoped vars
key = "base"; value = "/users"; enabled = true; secret = false

[scripts]                    # run around children (pre/test), reserved
pre_request = ""
test        = ""
```

### `*.req.toml` (HTTP request)

```toml
schema      = "sasin/request@1"
name        = "List users"
description = ""
method      = "GET"
url         = "{{base_url}}/users"

[[param]]                    # query params, kept in sync with url
key = "page";  value = "1";            enabled = true
[[param]]
key = "limit"; value = "{{page_size}}"; enabled = true

[[header]]
key = "Accept"; value = "application/json"; enabled = true

[auth]
type = "inherit"             # none | inherit | basic | bearer | apikey | oauth2
# basic:  user, pass
# bearer: token
# apikey: key, value, add_to = "header" | "query"
# oauth2: token, (grant config…)

[body]
mode = "none"                # none | raw | urlencoded | formdata | binary | graphql
# raw:        language = "json"|"text"|"xml"|"html"|"javascript"; text = '''...'''
# urlencoded: [[body.urlencoded]] key, value, enabled
# formdata:   [[body.formdata]]  key, kind = "text"|"file", value | src, enabled
# binary:     file = "relative/path.bin"
# graphql:    query = '''...''' ; variables = '''{ }'''

[settings]
timeout_ms       = 30000
follow_redirects = true
verify_tls       = true
use_cookie_jar   = true
# proxy = "http://…"   (optional)

[scripts]
pre_request = '''
pm.environment.set("ts", Date.now())
'''
test = '''
pm.test("200 OK", () => pm.response.to.have.status(200))
'''
```

Large/binary bodies: store the payload in a sidecar file referenced by `body.file` /
`formdata.src` (relative path) rather than inlining — keeps diffs sane and supports non-UTF-8.

### `*.ws.toml` (WebSocket request)

```toml
schema       = "sasin/websocket@1"
name         = "Chat"
url          = "wss://{{host}}/chat"
subprotocols = ["json"]

[[header]]
key = "Origin"; value = "https://{{host}}"; enabled = true

[auth]
type = "bearer"; token = "{{access_token}}"

[settings]
connect_timeout_ms = 5000
auto_reconnect     = false
verify_tls         = true

[[message]]                  # saved outbound messages
name = "ping"; kind = "json"; content = '''{"type":"ping"}'''
[[message]]
name = "raw-hello"; kind = "text"; content = "hello"
```

### `environments/*.toml`

```toml
schema = "sasin/environment@1"
name   = "dev"

[[variable]]
key = "base_url"; value = "https://dev.api.example.com"; enabled = true; secret = false
[[variable]]
key = "access_token"; value = ""; enabled = true; secret = true   # masked in UI, still on disk
```

`globals.toml` has the same shape (no env name semantics; lowest-priority scope).
**Secrets:** `secret = true` masks in the UI but values still land on disk. For real secret hygiene,
a later option is a gitignored `*.local.toml` overlay (out of scope now; reserve the idea).

## Cache (`.sasin-cache/`, derived, gitignored)

`bitcode` + `zstd` (reuse the existing codec from `persist/dataset.rs`). Never the source of truth;
rebuilt by scanning the tree on load when missing/stale.

| File | Holds |
|------|-------|
| `index.bc.zst` | Flattened node index (path, kind, name, method) for fast tree render + fuzzy search. |
| `session.bc.zst` | Open tabs, active tab, active environment, selection, split px. |
| `history.bc.zst` | Request history (now persisted — fixes B4). |
| `cookies.bc.zst` | Cookie jar (per workspace). |
| `responses/<hash>.bc.zst` | Last response per request (optional, for instant reopen). |

Staleness: compare tree mtimes / a content hash against the index; rebuild on mismatch.
A `notify`-based watcher (later phase) reloads the tree on external change (git pull, branch switch).

## Atomic writes & dirty tracking

- Per-file write: serialize → write `.<name>.tmp` → `fsync` → `rename` over target (existing pattern
  in `dataset.rs`/`workspace.rs`, lifted into `storage/save.rs`).
- Save only **dirty** nodes (per-tab dirty flag), not the whole tree, so git diffs stay minimal.
- Deletes/moves go through the storage layer so the index + open tabs stay consistent.
- Serialize maps/arrays in **stable order** (`preserve_order` already on `serde_json`; for `toml`
  preserve insertion order) so re-saving an unchanged request yields a **byte-identical** file → no
  spurious git diffs.

## Migration (one-time)

On opening a path: if it's a legacy `dataset.sasin` (or the app's old default dataset exists and no
`sasin.toml` is present), import → write a TOML tree beside it → switch the source of truth. Old file
left untouched (manual cleanup). Covered by `storage/migrate.rs` + a round-trip test.

## Invariants (tested)

1. **Round-trip**: `load(save(ws)) == ws` for every node kind.
2. **Stable bytes**: saving an unmodified workspace produces no file changes.
3. **Order fidelity**: `folder.toml`/manifest `order` round-trips; unknown/missing entries fall back
   to lexical, with new files appended.
4. **Path identity**: rename via storage API updates references; no dangling tabs/history.
5. **Cache is disposable**: deleting `.sasin-cache/` then loading reproduces identical in-memory state.
