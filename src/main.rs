use crate::{
    context::{Config, Context},
    server::DocsServer,
};
use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod client;
mod context;
mod server;
#[cfg(test)]
mod test_utils;
mod tools;
mod types;

pub(crate) const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Initialize tracing with two sinks:
///   - stderr (compact, ERROR+WARN by default) — for visibility during dev.
///     **Never** stdout: the MCP server speaks JSON-RPC over stdio, so any
///     stray byte to stdout corrupts the wire protocol.
///   - daily-rolled file under `<cache>/logs/` (verbose, INFO+ by default) —
///     persistent record for later inspection.
///
/// Both layers honour `RUST_LOG` if set. The returned `WorkerGuard` must
/// live for the program's lifetime so the non-blocking writer flushes.
fn init_tracing(config: &Config) -> Result<WorkerGuard> {
    let file_appender = rolling::Builder::new()
        .rotation(rolling::Rotation::DAILY)
        .filename_prefix(APP_NAME)
        .filename_suffix("log")
        .max_log_files(10)
        .build(&config.log_dir)?;
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let stderr_layer = fmt::layer()
        .compact()
        .with_writer(std::io::stderr)
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .with_default_directive(LevelFilter::WARN.into())
                .parse_lossy("")
        }));

    let file_layer = fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(file_writer)
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .parse_lossy(format!("{APP_NAME}=debug"))
        }));

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let _tracing_guard = init_tracing(&config)?;
    info!(log_dir = %config.log_dir.display(), "tracing initialized");

    let context = Context::new(config);

    let service = DocsServer::new(context)
        .serve(stdio())
        .await
        .inspect_err(|e| {
            error!(?e, "serving error");
        })?;

    service.waiting().await?;
    Ok(())
}
