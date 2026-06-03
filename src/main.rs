use anyhow::Result;
use counter::Counter;
use rmcp::{ServiceExt, transport::stdio};
use std::io;
use tracing::{error, info};
use tracing_subscriber::{self, EnvFilter};

mod counter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(io::stderr)
        .with_ansi(false)
        .init();

    info!("Starting MCP server");

    let service = Counter::new().serve(stdio()).await.inspect_err(|e| {
        error!(?e, "serving error");
    })?;

    service.waiting().await?;
    Ok(())
}
