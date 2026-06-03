use anyhow::Result;
use counter::Counter;
use rmcp::{ServiceExt, transport::stdio};
use std::io;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{self, EnvFilter, fmt, layer::SubscriberExt as _};

use crate::{config::Config, rustdoc_json::fetch_rustdoc_json};

mod config;
mod counter;
mod rustdoc_json;

pub(crate) const APP_NAME: &str = "docs-rs-mcp";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let config = Config::new()?;
    let path = fetch_rustdoc_json(&config, "axum", None).await?;
    dbg!(&path);

    info!("Starting MCP server");

    // let service = Counter::new().serve(stdio()).await.inspect_err(|e| {
    //     error!(?e, "serving error");
    // })?;

    // service.waiting().await?;
    Ok(())
}
