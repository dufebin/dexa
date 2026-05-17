use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use crate::service::Service;

#[derive(Parser)]
#[command(name = "hand", about = "Human-like desktop control via CLI or MCP")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Mouse(MouseCmd),
    Key(KeyCmd),
    /// Start the MCP server on stdio
    Mcp,
}

// ── mouse ────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct MouseCmd {
    #[command(subcommand)]
    pub subcmd: MouseSubCmd,
}

#[derive(Subcommand)]
pub enum MouseSubCmd {
    Move(MouseMoveArgs),
    Click(MouseClickArgs),
    Drag(MouseDragArgs),
    Scroll(MouseScrollArgs),
    Pos,
}

#[derive(Args)]
pub struct MouseMoveArgs {
    #[arg(long)]
    pub x: i32,
    #[arg(long)]
    pub y: i32,
    /// Duration in milliseconds (default: 300)
    #[arg(long, default_value_t = 300)]
    pub ms: u64,
    /// Movement mode: human (default) or fast
    #[arg(long, default_value = "human")]
    pub mode: String,
}

#[derive(Args)]
pub struct MouseClickArgs {
    #[arg(long)]
    pub x: i32,
    #[arg(long)]
    pub y: i32,
    /// Button: left (default) or right
    #[arg(long, default_value = "left")]
    pub button: String,
    /// Double-click
    #[arg(long, default_value_t = false)]
    pub double: bool,
}

#[derive(Args)]
pub struct MouseDragArgs {
    #[arg(long)]
    pub x1: i32,
    #[arg(long)]
    pub y1: i32,
    #[arg(long)]
    pub x2: i32,
    #[arg(long)]
    pub y2: i32,
    #[arg(long, default_value_t = 400)]
    pub ms: u64,
    #[arg(long, default_value = "human")]
    pub mode: String,
}

#[derive(Args)]
pub struct MouseScrollArgs {
    #[arg(long)]
    pub delta: i32,
}

// ── key ──────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct KeyCmd {
    #[command(subcommand)]
    pub subcmd: KeySubCmd,
}

#[derive(Subcommand)]
pub enum KeySubCmd {
    Type(KeyTypeArgs),
    Tap(KeyTapArgs),
    Combo(KeyComboArgs),
    /// Write text to clipboard and send Ctrl+V (works for CJK/Unicode)
    Paste(KeyPasteArgs),
}

#[derive(Args)]
pub struct KeyTypeArgs {
    #[arg(long)]
    pub text: String,
    #[arg(long, default_value = "human")]
    pub mode: String,
}

#[derive(Args)]
pub struct KeyTapArgs {
    #[arg(long)]
    pub key: String,
}

#[derive(Args)]
pub struct KeyComboArgs {
    #[arg(long)]
    pub keys: String,
}

#[derive(Args)]
pub struct KeyPasteArgs {
    #[arg(long)]
    pub text: String,
}

// ── dispatch ─────────────────────────────────────────────────────────────────

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Mcp => crate::mcp::run_mcp_server().await,
        Commands::Mouse(cmd) => {
            let mut svc = Service::new();
            run_mouse(cmd, &mut svc).await
        }
        Commands::Key(cmd) => {
            let mut svc = Service::new();
            run_key(cmd, &mut svc).await
        }
    }
}

async fn run_mouse(cmd: MouseCmd, svc: &mut Service) -> Result<()> {
    use MouseSubCmd::*;
    match cmd.subcmd {
        Move(a) => {
            let mode = a.mode.parse()?;
            svc.mouse_move(a.x, a.y, a.ms, mode).await
        }
        Click(a) => svc.mouse_click(a.x, a.y, &a.button, a.double).await,
        Drag(a) => {
            let mode = a.mode.parse()?;
            svc.mouse_drag(a.x1, a.y1, a.x2, a.y2, a.ms, mode).await
        }
        Scroll(a) => svc.mouse_scroll(a.delta).await,
        Pos => {
            let (x, y) = svc.mouse_pos().await?;
            println!("{{\"x\":{x},\"y\":{y}}}");
            Ok(())
        }
    }
}

async fn run_key(cmd: KeyCmd, svc: &mut Service) -> Result<()> {
    use KeySubCmd::*;
    match cmd.subcmd {
        Type(a) => {
            let mode = a.mode.parse()?;
            svc.key_type(&a.text, mode).await
        }
        Tap(a) => svc.key_tap(&a.key).await,
        Combo(a) => svc.key_combo(&a.keys).await,
        Paste(a) => svc.paste_text(&a.text).await,
    }
}
