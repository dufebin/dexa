use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::Read as IoRead;
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
    /// LLM text operations: distill contacts, generate replies, distill self persona
    Llm {
        #[command(subcommand)]
        cmd: LlmCmd,
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

#[derive(Subcommand)]
pub enum LlmCmd {
    /// Distill a contact's chat history into a JSON profile.
    /// Reads JSON from stdin: {"contact": "name", "messages": "chat text"}
    DistillContact,
    /// Generate a WeChat reply for an incoming message.
    /// Reads JSON from stdin: {"sender": "name", "content": "msg", "history": "text", "profile": "json|null", "max_len": 80}
    GenerateReply,
    /// Distill the user's own messages into a persona Markdown document.
    /// Reads JSON from stdin: {"messages": "chat text"}
    DistillSelf,
}

// ── runner ────────────────────────────────────────────────────────────────────

pub async fn run(cli: Cli, service: Arc<Service>) -> Result<()> {
    match cli.command {
        Commands::Screen { cmd } => run_screen(cmd, service).await,
        Commands::Memory { cmd } => run_memory(cmd, service).await,
        Commands::App { cmd } => run_app(cmd, service).await,
        Commands::Llm { cmd } => run_llm(cmd, service).await,
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

async fn run_llm(cmd: LlmCmd, service: Arc<Service>) -> Result<()> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("failed to read stdin")?;
    let val: serde_json::Value =
        serde_json::from_str(&input).context("stdin is not valid JSON")?;

    match cmd {
        LlmCmd::DistillContact => {
            let contact  = val["contact"].as_str().context("missing 'contact' in input")?;
            let messages = val["messages"].as_str().context("missing 'messages' in input")?;
            let result = service.llm_distill_contact(contact, messages).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        LlmCmd::GenerateReply => {
            let sender   = val["sender"].as_str().context("missing 'sender' in input")?;
            let content  = val["content"].as_str().context("missing 'content' in input")?;
            let history  = val["history"].as_str().unwrap_or("");
            let profile  = val["profile"].as_str();
            let max_len  = val["max_len"].as_u64().unwrap_or(80) as usize;
            let result = service
                .llm_generate_reply(sender, content, history, profile, max_len)
                .await?;
            // Print only the reply text for easy subprocess consumption.
            if let Some(reply) = result["reply"].as_str() {
                println!("{reply}");
            } else {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        LlmCmd::DistillSelf => {
            let messages = val["messages"].as_str().context("missing 'messages' in input")?;
            let result = service.llm_distill_self(messages).await?;
            // Print only the markdown for easy file-write consumption.
            if let Some(md) = result["markdown"].as_str() {
                println!("{md}");
            } else {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
    }
    Ok(())
}
