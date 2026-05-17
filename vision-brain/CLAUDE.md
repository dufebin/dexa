# Vision Brain — CLAUDE.md

## Module Map

| File | Purpose |
|---|---|
| `src/main.rs` | Binary entry; clap parse → `cli::run` |
| `src/cli.rs` | `screen / memory / app / mcp` subcommand tree |
| `src/service.rs` | `Service` — shared business logic (CLI + MCP) |
| `src/capture.rs` | `capture_primary()` → base64 PNG |
| `src/vision.rs` | `analyze(b64, task)` → `AnalyzeResult`; `llm_match_app()` for app open |
| `src/memory.rs` | `Memory` — SQLite CRUD via `spawn_blocking` |
| `src/apps.rs` | App discovery, fuzzy match (`fuzzy_find`), `launch_app` |
| `src/mcp.rs` | `VisionServer` — rmcp 1.x stdio server, 8 tools |

## Design Constraints

- `Service` methods take `&self`; no `&mut` needed — all mutation goes through SQLite or spawn_blocking closures
- `Memory` and `Service` are `Send + Sync` so they can be wrapped in `Arc<>` and shared between CLI and MCP server
- `rusqlite::Connection` is NOT `Send` — every DB operation opens a fresh connection inside `spawn_blocking`
- CLI and MCP share the same `Arc<Service>` instance

## screenshots crate type mismatch (IMPORTANT)

`screenshots` 0.8 bundles its own copy of the `image` crate. `screen.capture()` returns
`screenshots::image::ImageBuffer<Rgba<u8>, Vec<u8>>`, NOT `image::DynamicImage`.

Convert via raw bytes:
```rust
let (w, h) = (img.width(), img.height());
let dyn_img = image::DynamicImage::ImageRgba8(
    image::RgbaImage::from_raw(w, h, img.into_raw()).unwrap()
);
```

## UTF-8 string slicing (IMPORTANT)

Never slice task/query strings by byte index. Use `.chars().take(N).collect::<String>()` to
avoid panics when Chinese characters cross the byte boundary.

## rmcp 1.x Patterns

```rust
// 1. new() MUST be in a SEPARATE impl block from #[tool_router]
impl VisionServer {
    pub fn new(...) -> Self { ... }
}

// 2. Import `tool` explicitly
use rmcp::{tool, tool_handler, tool_router, ...};

// 3. Parameters wrapper path
use rmcp::handler::server::wrapper::Parameters;

// 4. ServerInfo is #[non_exhaustive] — use field mutation
let mut info = ServerInfo::default();
info.instructions = Some("...".into());
info.capabilities = ServerCapabilities::builder().enable_tools().build();
```

## LLM Provider

`LLM_PROVIDER` env var selects provider. Supports `anthropic` (default) and `openai`
(also accepts any OpenAI-compatible endpoint via `LLM_API_URL`).

| Variable | Default | Description |
|---|---|---|
| `LLM_PROVIDER` | `anthropic` | `anthropic` or `openai` |
| `LLM_API_KEY` | — | API key / Bearer token |
| `LLM_MODEL` | provider default | Model name |
| `LLM_API_URL` | provider default | Override base URL (full completions URL) |

Anthropic requires headers `x-api-key` and `anthropic-version: 2023-06-01`.
OpenAI uses `Authorization: Bearer <key>`.

Retry loop (up to 3) on JSON parse failure to handle malformed model output.

## App Discovery (`apps.rs`)

### macOS

Scans `/Applications`, `/System/Applications`, and `~/Applications` for `.app` bundles.

For each bundle, reads `Contents/Info.plist` for the English name, then tries
`zh-Hans.lproj/InfoPlist.strings` (and zh-Hant / zh_CN / zh variants) for a localized name.

`InfoPlist.strings` exists in three formats — all handled:
1. Binary plist (`plist` crate)
2. UTF-16 LE with BOM (`0xFF 0xFE`)
3. UTF-8 plain text

Keys can be quoted (`"CFBundleDisplayName" = "微信";`) or unquoted (`CFBundleDisplayName = "飞书";`).

`AppInfo.name` = localized name (e.g. `微信`, `飞书`).
`AppInfo.fs_name` = English/bundle name (e.g. `WeChat`, `Lark`).
Both are checked during fuzzy match.

Launch: `open <path>` via `std::process::Command`.

### Windows

Scans **Start Menu `.lnk` shortcut folders** — NOT `Program Files`.

| Path | Scope |
|---|---|
| `C:\ProgramData\Microsoft\Windows\Start Menu\Programs\` | System-wide |
| `%APPDATA%\Microsoft\Windows\Start Menu\Programs\` | Current user |

Launch: `cmd /C start "" <path.lnk>`.

### Matching pipeline (`service::app_open`)

```
query
  │
  ▼  normalize: lowercase + strip whitespace
  │
  ├─ 1. exact match (query == name or query == fs_name)
  │
  ├─ 2. name/fs_name contains query
  │
  ├─ 3. query contains name/fs_name  ("打开微信" contains "微信")
  │
  ├─ 4. jaro-winkler ≥ 0.75 on name + fs_name
  │
  └─ 5. LLM fallback  (only when LLM_API_KEY is set)
           sends compact app list + query → model returns exact path
```

Exact-match priority (step 1) prevents shorter names (e.g. "WeChat") from losing to
longer ones that happen to contain the same substring (e.g. "WeChatWebDevTools").

### Platform guards

All platform-specific functions in `apps.rs` use `#[cfg(target_os = "...")]`.
The `#[cfg(not(any(...)))]` fallback returns an empty list and an unsupported-platform
error so the crate compiles on Linux without dead-code warnings.

## Adding a New MCP Tool

1. Add param struct with `#[derive(Deserialize, schemars::JsonSchema)]`
2. Add method to `service.rs` returning `Result<serde_json::Value>`
3. Add `#[tool(...)]` method in `mcp.rs` `#[tool_router]` impl block
4. Add subcommand to `cli.rs` and dispatch to the same service method

## Dependencies

| Crate | Version | Why |
|---|---|---|
| rmcp | 1 | MCP stdio server |
| screenshots | 0.8 | screen capture |
| image | 0.25 | PNG encoding |
| rusqlite | 0.31 | SQLite memory store |
| reqwest | 0.12 | LLM HTTP calls |
| base64 | 0.22 | PNG → base64 |
| chrono | 0.4 | timestamps |
| tokio | 1 | async runtime |
| anyhow | 1 | error handling |
| clap | 4 | CLI argument parsing |
| strsim | 0.11 | jaro-winkler fuzzy match for app names |
| plist | 1 | read macOS `Info.plist` (binary + XML) |
