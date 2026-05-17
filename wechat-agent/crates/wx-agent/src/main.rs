mod cmd_distill;
mod cmd_send;
mod cmd_watch;
mod config;
mod wechat_ui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name    = "wx-agent",
    version = "0.1.0",
    about   = "WeChat automation agent: distill contacts, auto-reply, send messages"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Distill a contact's persona or your own self-persona
    Distill {
        #[command(subcommand)]
        sub: DistillSub,
    },
    /// Start the message watcher and auto-reply daemon
    Watch {
        /// Fully automatic mode (no confirmation prompt)
        #[arg(long)]
        auto: bool,
    },
    /// Send a single message to a contact (for testing)
    Send {
        /// Contact display name
        contact: String,
        /// Message text
        message: String,
    },
    /// Show your contact profile for a given name
    Profile {
        /// Contact name
        name: String,
    },
}

#[derive(Subcommand)]
enum DistillSub {
    /// Analyze and store a contact's persona
    Contact {
        /// Contact display name (as shown in WeChat)
        name: String,
    },
    /// Distill your own persona into a Hermes/OpenClaw SKILL.md
    Self_ {
        /// Optionally limit to one conversation for self-analysis
        #[arg(long)]
        from: Option<String>,
    },
    /// List all distilled contacts
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let cfg = config::Config::load()?;

    match cli.command {
        Commands::Distill { sub } => match sub {
            DistillSub::Contact { name } => {
                cmd_distill::run_contact(&name, &cfg).await?;
            }
            DistillSub::Self_ { from } => {
                cmd_distill::run_self(from.as_deref(), &cfg).await?;
            }
            DistillSub::List => {
                cmd_distill::run_list(&cfg).await?;
            }
        },
        Commands::Watch { auto } => {
            cmd_watch::run(auto, &cfg).await?;
        }
        Commands::Send { contact, message } => {
            cmd_send::run(&contact, &message, &cfg).await?;
        }
        Commands::Profile { name } => {
            use wx_core::Database;
            let db = Database::open(&config::default_db_path()).await?;
            match db.get_profile(&name).await? {
                None => println!("（未找到 「{name}」 的画像，请先运行 distill contact）"),
                Some(p) => {
                    println!("=== 联系人画像：{name} ===");
                    println!("概括  ：{}", p.summary);
                    println!("关系  ：{}", p.relationship);
                    println!("风格  ：{}", p.communication_style);
                    println!("话题  ：{}", p.topics.join("、"));
                    println!("情感  ：{}", p.emotional_pattern);
                    println!("策略  ：{}", p.response_strategy);
                    println!("更新  ：{}", p.updated_at);
                }
            }
        }
    }

    Ok(())
}
