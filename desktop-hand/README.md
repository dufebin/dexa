# desktop-hand

Human-like desktop control — mouse and keyboard automation with natural bezier movement and variable typing delays.

Dual-mode: **MCP server** (stdio) + **CLI**.

## Features

- Non-linear mouse movement via cubic bezier curves with ease-in-out timing
- Random ±1–3 px jitter per waypoint
- Char-by-char typing with randomized delays (30–150 ms per char, 100–400 ms word pause, 300–800 ms sentence pause)
- Optional typo simulation (~2% probability, backspace to correct)
- `human` (default) and `fast` modes for all operations
- CLI and MCP share the same service layer — no duplicated logic

## Requirements

- macOS: grant **Accessibility** permission to the terminal or app running `hand`
  (`System Settings → Privacy & Security → Accessibility`)

## Build

```bash
cargo build --release
# binary: target/release/hand
```

## CLI

```bash
# Mouse
hand mouse move --x 500 --y 300
hand mouse move --x 500 --y 300 --ms 200 --mode fast
hand mouse click --x 500 --y 300
hand mouse click --x 500 --y 300 --button right --double
hand mouse drag --x1 100 --y1 100 --x2 400 --y2 400
hand mouse drag --x1 100 --y1 100 --x2 400 --y2 400 --ms 600
hand mouse scroll --delta -3
hand mouse pos

# Keyboard
hand key type --text "hello world"
hand key type --text "fast text" --mode fast
hand key tap --key Return
hand key tap --key F5
hand key combo --keys "ctrl+c"
hand key combo --keys "cmd+shift+4"

# MCP server (stdio)
hand mcp
```

### Supported key names

`ctrl`, `alt`, `shift`, `meta` / `cmd`, `return` / `enter`, `esc`, `tab`,
`backspace`, `delete`, `space`, `up`, `down`, `left`, `right`,
`home`, `end`, `pageup`, `pagedown`, `f1`–`f12`,
or any single character (`a`, `1`, `$`, …)

## MCP Server

Start with `hand mcp`. Connects via **stdio** transport.

### Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `mouse_move` | `x`, `y`, `ms?`, `mode?` | Move mouse to coordinates |
| `mouse_click` | `x`, `y`, `button?`, `double?` | Click at coordinates |
| `mouse_drag` | `x1`, `y1`, `x2`, `y2`, `ms?`, `mode?` | Drag from one point to another |
| `mouse_scroll` | `delta` | Scroll wheel (positive = up) |
| `mouse_pos` | — | Returns `{"x": int, "y": int}` |
| `key_type` | `text`, `mode?` | Type a string |
| `key_tap` | `key` | Tap a single key by name |
| `key_combo` | `keys` | Press a key combination, e.g. `"ctrl+c"` |

`mode` defaults to `"human"` for all tools. Set `"fast"` to skip delays.

### Claude Desktop config

```json
{
  "mcpServers": {
    "desktop-hand": {
      "command": "/path/to/hand",
      "args": ["mcp"]
    }
  }
}
```

## Architecture

```
CLI args ──► cli.rs ──┐
                       ├──► service.rs ──► behavior (Mode) ──► executor.rs ──► enigo
MCP JSON ──► mcp.rs ──┘         │
                           smooth.rs (bezier path)
                           human.rs  (timing / jitter)
```

- **executor.rs** — stateless; each operation runs in `spawn_blocking` with a fresh `Enigo`
- **service.rs** — validates inputs, generates paths/sequences, dispatches to executor
- **smooth.rs** — cubic bezier + ease-in-out timing
- **human.rs** — randomized delays and jitter

## Logging

Set `RUST_LOG=debug` to see per-operation traces (written to stderr, safe for MCP stdio use).

## License

MIT
