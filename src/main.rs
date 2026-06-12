use crate::{context::Context, server::DocsServer};
use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::{self, EnvFilter};

mod client;
mod context;
mod server;
#[cfg(test)]
mod test_utils;
mod tools;
mod types;

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

    let config = Context::from_env()?;

    let service = DocsServer::new(config)
        .serve(stdio())
        .await
        .inspect_err(|e| {
            error!(?e, "serving error");
        })?;

    service.waiting().await?;
    Ok(())
}
