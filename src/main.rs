use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::{self, EnvFilter};

use crate::server::DocsServer;

mod client;
mod config;
mod docs_rs;
mod rustdoc_json;
mod server;
mod tools;

pub(crate) const APP_NAME: &str = env!("CARGO_PKG_NAME");

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

    let service = DocsServer::new().serve(stdio()).await.inspect_err(|e| {
        error!(?e, "serving error");
    })?;

    service.waiting().await?;
    Ok(())
}
