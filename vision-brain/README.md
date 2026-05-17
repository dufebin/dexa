# Vision Brain

A pure Rust CLI + MCP stdio server for screen perception, persistent memory, and natural-language app control.

## Features

- **Screen capture** — captures the primary display as a base64 PNG
- **LLM analysis** — sends screenshots to Claude/OpenAI and returns structured action steps
- **Persistent memory** — stores and retrieves action step sequences in SQLite (with hit counting)
- **App launcher** — discovers installed apps, opens them via natural language ("打开微信", "open Chrome")
- **MCP server** — exposes all features as MCP tools for Claude Desktop / other MCP clients

## Demo — Open Feishu and send a message

```bash
# 1. Open Feishu via natural-language query
LLM_PROVIDER=openai \
LLM_API_KEY="300001758:ff3c679310effcb5113c9e5a7c5d0c7b" \
LLM_API_URL="http://ai-service.tal.com/openai-compatible/v1/chat/completions" \
LLM_MODEL="gemini-3.1-pro" \
vision-brain app open --query "飞书"
# → {"app":"飞书","method":"fuzzy","ok":true}

# 2. Capture the screen and ask the LLM what to click
LLM_PROVIDER=openai \
LLM_API_KEY="300001758:ff3c679310effcb5113c9e5a7c5d0c7b" \
LLM_API_URL="http://ai-service.tal.com/openai-compatible/v1/chat/completions" \
LLM_MODEL="gemini-3.1-pro" \
vision-brain screen analyze \
  --task "Find the chat named 'hermes' and send a message: hello"
# → {"steps":[
#     {"action":"click","target":"hermes 机器人","x":338,"y":366},
#     {"action":"click","target":"message input box","x":653,"y":863},
#     {"action":"type","target":"message input box","text":"hello"}
#   ]}

# 3. Execute steps with desktop-hand
hand mouse click --x 338 --y 366
hand mouse click --x 653 --y 863
hand key type --text "hello"
hand key tap --key "Return"
```

## Build

```bash
cargo build --release
```

No Node.js or npm required.

## CLI Usage

```bash
# App control
vision-brain app open --query "打开微信"
vision-brain app open --query "飞书"
vision-brain app list

# Screen
vision-brain screen capture
vision-brain screen analyze --task "open File menu and click Save"

# Memory
vision-brain memory list
vision-brain memory get --key "login-flow"
vision-brain memory set --key "login-flow" --steps '[{"action":"click","target":"login button"}]'
vision-brain memory delete --key "login-flow"

# MCP stdio server
vision-brain mcp
```

## MCP (Claude Desktop)

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "vision-brain": {
      "command": "/path/to/vision-brain",
      "args": ["mcp"],
      "env": {
        "LLM_PROVIDER": "anthropic",
        "LLM_API_KEY": "sk-ant-...",
        "LLM_MODEL": "claude-opus-4-5"
      }
    }
  }
}
```

### MCP Tools

| Tool | Input | Output |
|---|---|---|
| `screen_capture` | — | `{"base64": "..."}` |
| `screen_analyze` | `task`, `screenshot?` | `{"steps": [...]}` |
| `memory_get` | `key` | stored steps or `null` |
| `memory_set` | `key`, `steps` | `{"ok": true}` |
| `memory_delete` | `key` | `{"ok": true/false}` |
| `memory_list` | — | `[{key, hits, updated_at}, ...]` |
| `app_list` | — | `[{name, fs_name, path}, ...]` |
| `app_open` | `query` | `{ok, app, method}` |

### Step schema

```json
{
  "action": "click",
  "target": "File menu",
  "x": 45,
  "y": 22,
  "text": null,
  "duration_ms": null
}
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `LLM_PROVIDER` | `anthropic` | `anthropic` or `openai` |
| `LLM_API_KEY` | — | API key |
| `LLM_MODEL` | `claude-opus-4-5` | Model name |
| `LLM_API_URL` | provider default | Override base URL |
| `RUST_LOG` | — | Log level, e.g. `debug` (stderr) |

## Architecture

```
src/
├── main.rs      — entry point; clap parse → cli::run
├── cli.rs       — command definitions (screen / memory / app / mcp)
├── service.rs   — shared business logic (CLI + MCP)
├── mcp.rs       — rmcp 1.x stdio server, 8 tools
├── capture.rs   — screenshots-rs → base64 PNG
├── vision.rs    — LLM calls (Anthropic/OpenAI) + app matching
├── memory.rs    — SQLite CRUD via rusqlite + spawn_blocking
└── apps.rs      — app discovery, fuzzy match, launch
```

## App Discovery

### macOS

Scans the following directories for `.app` bundles:

| Directory | Scope |
|---|---|
| `/Applications` | User-installed apps |
| `/System/Applications` | System apps (Calculator, Safari, etc.) |
| `~/Applications` | Per-user installs |

For each bundle, reads `Contents/Info.plist` then `zh-Hans.lproj/InfoPlist.strings` to get
the localized display name (e.g. `微信` for WeChat). Supports binary plist, XML plist,
UTF-16 LE, and UTF-8 strings files.

Launch: `open /Applications/WeChat.app`

### Windows

Scans **Start Menu shortcut (`.lnk`) folders** — the same source as the Windows Start menu.
Results are clean (no uninstallers or internal binaries). UWP/Store apps are included if
they have a Start Menu shortcut.

| Directory | Scope |
|---|---|
| `C:\ProgramData\Microsoft\Windows\Start Menu\Programs\` | System-wide |
| `%APPDATA%\Microsoft\Windows\Start Menu\Programs\` | Current user |

Launch: `cmd /C start "" <path.lnk>`

### Matching flow

```
query
  │
  ▼  normalize (lowercase, strip whitespace)
  │
  ├─ 1. exact match
  ├─ 2. name/fs_name contains query
  ├─ 3. query contains name/fs_name  ("打开微信" contains "微信")
  ├─ 4. jaro-winkler similarity ≥ 0.75
  └─ 5. LLM fallback (claude-haiku / gpt-4o-mini, only if LLM_API_KEY set)
```

## Database

SQLite at `~/.vision-brain/memory.db`.

```sql
CREATE TABLE memories (
    key        TEXT PRIMARY KEY,
    steps      TEXT NOT NULL,
    hits       INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
```
