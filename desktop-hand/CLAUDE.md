# CLAUDE.md — desktop-hand

## Build & check

```bash
cargo check          # fast type-check
cargo build          # debug build
cargo build --release
cargo clippy         # lints
```

Binary name: `hand` (set in `[[bin]]` in Cargo.toml).

## Module map

| File | Role |
|------|------|
| `main.rs` | Entry point. Init tracing (stderr), parse CLI, dispatch. |
| `cli.rs` | All clap definitions. `run()` dispatches to `service.rs` or `mcp::run_mcp_server()`. |
| `mcp.rs` | rmcp 1.x MCP server. `HandServer` holds `Arc<Mutex<Service>>`. |
| `service.rs` | Core logic. Validates inputs, generates paths/sequences, calls executor. |
| `behavior.rs` | `Mode` enum (`Human` / `Fast`). `InputBehavior` trait (documentation, not dyn dispatch). |
| `executor.rs` | Stateless enigo wrapper. Every method runs in `tokio::task::spawn_blocking`. |
| `human.rs` | Pure functions for random delays and jitter. No side effects. |
| `smooth.rs` | Cubic bezier + ease-in-out timing → `Vec<Waypoint>`. No side effects. |

## Key design constraints

- **No shared state between calls** — `Executor` creates a fresh `enigo::Enigo` inside each `spawn_blocking` closure. This avoids `Send` issues with enigo on macOS.
- **All randomness lives in service.rs** — `executor.rs` and `smooth.rs` are deterministic given their inputs.
- **CLI and MCP share service.rs** — no logic duplication.
- **Tracing writes to stderr** — safe with MCP stdio transport.

## rmcp 1.x patterns

```rust
// Parameters must come from wrapper, not tool:
use rmcp::handler::server::wrapper::Parameters;

// new() must be in a SEPARATE impl block from #[tool_router]:
impl HandServer { pub fn new() -> Self { ... } }
#[tool_router]
impl HandServer { #[tool(description = "...")] async fn my_tool(...) }

// ServerInfo is #[non_exhaustive]; use field mutation:
let mut info = ServerInfo::default();
info.instructions = Some("...".into());
info.capabilities = ServerCapabilities::builder().enable_tools().build();
```

## enigo 0.2 API notes

```rust
use enigo::{Enigo, Settings, Mouse, Keyboard, Button, Axis, Direction, Key, Coordinate};

let mut en = Enigo::new(&Settings::default())?;
en.move_mouse(x, y, Coordinate::Abs)?;
en.button(Button::Left, Direction::Click)?;   // Direction::{Press,Release,Click}
en.scroll(delta, Axis::Vertical)?;
en.location()?;                                // -> (i32, i32)
en.key(Key::Unicode('a'), Direction::Click)?;
en.fast_text("hello")?;                        // returns Option<()>; None = unsupported
```

Supported `Key` variants: `Control`, `Alt`, `Shift`, `Meta`, `Return`, `Escape`, `Tab`, `Backspace`, `Delete`, `Space`, `UpArrow`, `DownArrow`, `LeftArrow`, `RightArrow`, `Home`, `End`, `PageUp`, `PageDown`, `F1`–`F12`, `Unicode(char)`.

## Adding a new tool

1. Add params struct (with `#[derive(Debug, Deserialize, schemars::JsonSchema)]`) in `mcp.rs`
2. Add async method with `#[tool(description = "...")]` inside the `#[tool_router]` impl block
3. Add the business logic method to `service.rs`
4. Add a CLI subcommand in `cli.rs` that calls the same `service.rs` method

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| enigo | 0.2 | Mouse/keyboard control |
| rmcp | 1.x | MCP server (tool_router macros) |
| clap | 4 | CLI argument parsing |
| rand | 0.8 (`small_rng`) | Randomized timing and jitter |
| tokio | 1 (full) | Async runtime |
| anyhow | 1 | Error handling |
| tracing | 0.1 | Structured logging |
