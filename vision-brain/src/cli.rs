use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;

use crate::service::Service;

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "vision-brain", about = "Screen perception, memory, and app control")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Screen capture and analysis
    Screen {
        #[command(subcommand)]
        cmd: ScreenCmd,
    },
    /// Persistent action memory
    Memory {
        #[command(subcommand)]
        cmd: MemoryCmd,
    },
    /// App discovery and launch
    App {
        #[command(subcommand)]
        cmd: AppCmd,
    },
    /// Run as MCP stdio server
    Mcp,
}

#[derive(Subcommand)]
pub enum ScreenCmd {
    /// Capture primary screen, print base64 PNG
    Capture,
    /// Capture screen and analyze with LLM
    Analyze {
        /// Goal description, e.g. "open File menu and click Save"
        #[arg(short, long)]
        task: String,
    },
}

#[derive(Subcommand)]
pub enum MemoryCmd {
    /// List all stored memory keys
    List,
    /// Get stored steps by key
    Get {
        #[arg(short, long)]
        key: String,
    },
    /// Store steps under a key (steps as JSON string)
    Set {
        #[arg(short, long)]
        key: String,
        #[arg(short, long)]
        steps: String,
    },
    /// Delete a memory key
    Delete {
        #[arg(short, long)]
        key: String,
    },
}

#[derive(Subcommand)]
pub enum AppCmd {
    /// List all discovered applications
    List,
    /// Open an app by natural language query
    Open {
        /// e.g. "打开微信", "open chrome", "launch calculator"
        #[arg(short, long)]
        query: String,
    },
}

// ── runner ────────────────────────────────────────────────────────────────────

pub async fn run(cli: Cli, service: Arc<Service>) -> Result<()> {
    match cli.command {
        Commands::Screen { cmd } => run_screen(cmd, service).await,
        Commands::Memory { cmd } => run_memory(cmd, service).await,
        Commands::App { cmd } => run_app(cmd, service).await,
        Commands::Mcp => crate::mcp::run_mcp_server(service).await,
    }
}

async fn run_screen(cmd: ScreenCmd, service: Arc<Service>) -> Result<()> {
    match cmd {
        ScreenCmd::Capture => {
            let v = service.screen_capture().await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        ScreenCmd::Analyze { task } => {
            let v = service.screen_analyze(&task, None).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
    }
    Ok(())
}

async fn run_memory(cmd: MemoryCmd, service: Arc<Service>) -> Result<()> {
    match cmd {
        MemoryCmd::List => {
            let v = service.memory_list().await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        MemoryCmd::Get { key } => {
            let v = service.memory_get(key).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        MemoryCmd::Set { key, steps } => {
            let steps_val: serde_json::Value = serde_json::from_str(&steps)?;
            let v = service.memory_set(key, steps_val).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        MemoryCmd::Delete { key } => {
            let v = service.memory_delete(key).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
    }
    Ok(())
}

async fn run_app(cmd: AppCmd, service: Arc<Service>) -> Result<()> {
    match cmd {
        AppCmd::List => {
            let v = service.app_list().await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        AppCmd::Open { query } => {
            let v = service.app_open(query).await?;
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
    }
    Ok(())
}
