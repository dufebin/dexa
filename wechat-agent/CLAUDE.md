# CLAUDE.md — wechat-agent

## Build & check

```bash
cargo check                  # fast type-check (all crates)
cargo build                  # debug build
cargo build --release        # release build
cargo clippy                 # lints
cargo test                   # run tests
```

Binary: `target/release/wx-agent`

## Workspace layout

```
wechat-agent/
├── Cargo.toml               # workspace root
├── config.toml              # runtime config (not compiled in)
└── crates/
    ├── wx-core/             # shared library (no bin)
    ├── wx-distill/          # distillation logic (no bin)
    └── wx-agent/            # binary: wx-agent
```

## Module map

### wx-core

| File | Role |
|------|------|
| `models.rs` | All shared data types: `WxMessage`, `WxSession`, `WxContact`, `ContactProfile`, `PendingMessage` |
| `wx_client.rs` | `WxClient` — thin async wrapper around `wx` CLI subprocess. Parses `--json` output. |
| `hand_client.rs` | `HandClient` — thin async wrapper around `hand` CLI subprocess. One method per `hand` subcommand. |
| `llm.rs` | `LlmClient` — calls Claude API via `reqwest`. Three public methods: `generate_reply`, `distill_contact`, `distill_self`. |
| `db.rs` | `Database` — SQLite via `sqlx`. Tables: `contact_profiles`, `pending_messages`. |
| `lib.rs` | Re-exports everything from the modules above. |

### wx-distill

| File | Role |
|------|------|
| `contact.rs` | `distill_contact()` — export messages → LLM → `ContactProfile` → return |
| `self_distill.rs` | `distill_self()` — export self-messages → LLM → write SKILL.md |
| `lib.rs` | Re-exports both functions. |

### wx-agent

| File | Role |
|------|------|
| `config.rs` | `Config` struct — deserializes `config.toml`. Platform-aware defaults for `search_key` and `activate_cmd`. |
| `wechat_ui.rs` | `send_message()` — 6-step WeChat UI automation: activate → search → select → type → send. Platform-aware `activate_wechat()`. |
| `cmd_distill.rs` | Handlers for `distill contact`, `distill self`, `distill list`. |
| `cmd_send.rs` | Handler for `send` (manual test send). |
| `cmd_watch.rs` | Handler for `watch` — poll loop, message queue, reply confirmation, send. |
| `main.rs` | `clap` CLI wiring. Maps subcommands to `cmd_*` handlers. |

## Key design decisions

- **Two external binaries, not libraries**: `wx` and `hand` are called as subprocesses. This keeps the crates decoupled and means wx-agent works regardless of wx-cli/desktop-hand internal changes.
- **Pending message queue**: `wx new-messages` consumes messages from wx-cli's internal state. We immediately write all incoming messages to SQLite with `status = 'pending'` before processing. This makes restarts safe — unprocessed messages survive a crash.
- **`require_profile = true` default**: The agent only auto-replies to contacts that have been explicitly distilled. This prevents accidental replies to strangers.
- **Platform shortcuts via `cfg!`**: `search_key` defaults to `cmd+f` on macOS and `ctrl+f` on Windows. Overridable in `config.toml`.
- **Haiku for replies, Sonnet for distillation**: Reply generation is high-frequency (one call per message), so Haiku keeps cost low. Distillation is a one-shot analysis where quality matters.

## wx-cli output format notes

wx-cli uses `--json` for JSON output. The exact field names may vary between versions.
`WxMessage` uses `#[serde(alias = "...")]` to handle common variations.

If wx-cli changes its output format, update the aliases in `wx-core/src/models.rs`.

Key fields that wx-cli is expected to provide:
- `sender` / `from` — who sent the message
- `content` / `text` — message body
- `timestamp` / `create_time` — unix timestamp
- `chat_type` — "private" | "group" | "official_account" | "folded"
- `is_self` / `isSender` — whether the message was sent by the local user
- `chat_name` / `chat` — which conversation (present in `new-messages`, may be absent in `history`)

## Adding a new command

1. Add a new `cmd_<name>.rs` in `wx-agent/src/`.
2. Add a `Commands::NewCmd { ... }` variant in `main.rs`.
3. Wire it up in the `match cli.command { ... }` block.
4. If new data needs to be persisted, add a table in `db.rs` (edit the `SCHEMA` constant and add typed methods).

## Adding a new LLM feature

1. Add a method to `LlmClient` in `wx-core/src/llm.rs`.
2. It should call `self.call(model, system, prompt, max_tokens)` and return `Result<String>`.
3. If the output needs structured parsing, parse JSON in the calling code, not in `llm.rs`.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| tokio | 1 | Async runtime |
| serde / serde_json | 1 | Serialization |
| reqwest | 0.12 | HTTP client for Claude API |
| sqlx | 0.8 | SQLite async driver |
| clap | 4 | CLI argument parsing |
| anyhow / thiserror | latest | Error handling |
| chrono | 0.4 | Timestamps |
| dirs | 5 | Platform home directory (`~/.wx-agent`) |
| toml | 0.8 | Config file parsing |
| tracing / tracing-subscriber | 0.1 / 0.3 | Structured logging (stderr) |

## External tools required at runtime

| Tool | Source | Used for |
|------|--------|---------|
| `wx` | [jackwener/wx-cli](https://github.com/jackwener/wx-cli) | Reading WeChat local databases |
| `hand` | `../desktop-hand` (this repo) | Mouse/keyboard UI automation |
