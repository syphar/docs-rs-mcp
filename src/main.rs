use anyhow::{Result, bail};
use counter::Counter;
use rmcp::{ServiceExt, transport::stdio};
use std::io;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{self, EnvFilter, fmt, layer::SubscriberExt as _};

use crate::{config::Config, docs_rs::get_docs_status, rustdoc_json::get_docs};

mod config;
mod counter;
mod docs_rs;
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

    let mut args = std::env::args().skip(1);
    let krate = args.next().unwrap();
    let req_version = args.next().unwrap_or("latest".to_string());

    let config = Config::new()?;

    let status = get_docs_status(&krate, &req_version).await?;
    info!(?status, "resolved version");

    if !status.doc_status {
        bail!("no docs");
    }

    let krate = get_docs(&config, &krate, &status.version).await?;
    // dbg!(&krate);

    // let service = Counter::new().serve(stdio()).await.inspect_err(|e| {
    //     error!(?e, "serving error");
    // })?;

    // service.waiting().await?;
    Ok(())
}
