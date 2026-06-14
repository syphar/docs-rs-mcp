use crate::{
    context::{Config, Context},
    server::DocsServer,
};
use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{Instrument as _, error, info, info_span, level_filters::LevelFilter};
use tracing_appender::rolling;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

mod client;
mod context;
mod errors;
mod server;
#[cfg(test)]
mod test_utils;
mod tools;
mod types;

pub(crate) const APP_NAME: &str = env!("CARGO_PKG_NAME");
const ENV_NAME: &str = "DOCS_RS_MCP_LOG";

/// Initialize tracing with two sinks:
///   - stderr (compact, WARN+ by default) — for visibility during dev.
///     **Never** stdout: the MCP server speaks JSON-RPC over stdio, so any
///     stray byte to stdout corrupts the wire protocol.
///   - daily-rolled file under `<cache>/logs/`, JSON, INFO+ by default —
///     persistent record for later inspection. The PID is part of the
///     filename so concurrent instances don't share a file. Each event is
///     flushed to disk immediately (see `FlushOnWrite`) so we survive
///     SIGKILL with the log intact.
///
/// Both layers honour `RUST_LOG` if set.
fn init_tracing(config: &Config) -> Result<()> {
    let pid = std::process::id();
    let file_appender = rolling::Builder::new()
        .rotation(rolling::Rotation::DAILY)
        .filename_prefix(format!("{APP_NAME}.{pid}"))
        .filename_suffix("log")
        .max_log_files(10)
        .build(&config.log_dir)?;

    let stderr_layer = fmt::layer()
        .compact()
        .with_writer(std::io::stderr)
        .with_filter(
            EnvFilter::builder()
                .with_env_var(ENV_NAME)
                .with_default_directive(LevelFilter::WARN.into())
                .from_env_lossy(),
        );

    let file_layer = fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(file_appender)
        // Emit one JSON line per span close, carrying `time.busy` /
        // `time.idle` durations. Combined with `#[tracing::instrument]` on
        // tool handlers, this gives you per-call timing data in the log.
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(
            EnvFilter::builder()
                .with_env_var(ENV_NAME)
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        );

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    init_tracing(&config)?;

    info!(
        cwd = std::env::current_dir()
            .map(|cwd| cwd.display().to_string())
            .ok(),
        mcp_version = env!("CARGO_PKG_VERSION"),
        "instance started"
    );

    info!(log_dir = %config.log_dir.display(), "tracing initialized");

    let context = Context::new(config);

    let pid = std::process::id();
    let span = info_span!("serve", pid);

    let service = DocsServer::new(context)
        .serve(stdio())
        .instrument(span)
        .await
        .inspect_err(|e| {
            error!(?e, "serving error");
        })?;

    let wait = info_span!("wait", pid);
    service.waiting().instrument(wait).await?;
    Ok(())
}
