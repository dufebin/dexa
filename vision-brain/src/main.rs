mod apps;
mod capture;
mod cli;
mod memory;
mod mcp;
mod service;
mod vision;

use clap::Parser;
use std::{path::PathBuf, sync::Arc};

fn main() {
    let parsed = cli::Cli::parse();
    init_tracing();
    tokio::runtime::Runtime::new()
        .expect("tokio runtime")
        .block_on(async {
            let svc = Arc::new(
                service::Service::new(&default_db_path()).expect("service init"),
            );
            cli::run(parsed, svc).await.expect("command failed");
        });
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();
}

fn default_db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vision-brain").join("memory.db")
}
