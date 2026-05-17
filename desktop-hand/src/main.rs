mod behavior;
mod cli;
mod executor;
mod human;
mod mcp;
mod service;
mod smooth;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = cli::Cli::parse();
    cli::run(cli).await
}
