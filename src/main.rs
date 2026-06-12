use crate::{
    context::{Config, Context},
    server::DocsServer,
};
use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use std::path::{Path, PathBuf};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

mod client;
mod context;
mod server;
#[cfg(test)]
mod test_utils;
mod tools;
mod types;

pub(crate) const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Initialize tracing with two sinks:
///   - stderr (compact, WARN+ by default) — for visibility during dev.
///     **Never** stdout: the MCP server speaks JSON-RPC over stdio, so any
///     stray byte to stdout corrupts the wire protocol.
///   - daily-rolled file under `<cache>/logs/`, JSON, INFO+ by default —
///     persistent record for later inspection. The PID is part of the
///     filename so concurrent instances don't share a file.
///
/// Both layers honour `RUST_LOG` if set. The returned `WorkerGuard` must
/// live for the program's lifetime so the non-blocking writer flushes.
fn init_tracing(config: &Config) -> Result<WorkerGuard> {
    let pid = std::process::id();
    let file_appender = rolling::Builder::new()
        .rotation(rolling::Rotation::DAILY)
        .filename_prefix(format!("{APP_NAME}.{pid}"))
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
        // Emit one JSON line per span close, carrying `time.busy` /
        // `time.idle` durations. Combined with `#[tracing::instrument]` on
        // tool handlers, this gives you per-call timing data in the log.
        .with_span_events(FmtSpan::CLOSE)
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

/// Walk up from `start` looking for the nearest `Cargo.toml`. Returns the
/// directory it sits in.
fn find_project_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists())
        .map(Path::to_path_buf)
}

/// Parse the nearest `Cargo.toml` for the package name. Falls through to
/// `None` for workspace-only manifests (no `[package]`) or parse failures.
fn project_name(start: &Path) -> Option<String> {
    let root = find_project_root(start)?;
    let manifest = cargo_manifest::Manifest::from_path(root.join("Cargo.toml")).ok()?;
    manifest.package.map(|p| p.name)
}

/// Emit one INFO event with everything that's static for this process —
/// the AI/operator reading the log later can pin a session to its host,
/// project, MCP version, and host target without grepping.
fn log_instance_banner() {
    let cwd = std::env::current_dir().ok();
    let cwd_str = cwd.as_ref().map(|p| p.display().to_string());
    let project_root = cwd.as_ref().and_then(|c| find_project_root(c));
    let project_root_str = project_root.as_ref().map(|p| p.display().to_string());
    let project = cwd.as_ref().and_then(|c| project_name(c));
    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok();

    info!(
        pid = std::process::id(),
        cwd = cwd_str.as_deref(),
        project = project.as_deref(),
        project_root = project_root_str.as_deref(),
        hostname = %hostname,
        user = user.as_deref(),
        build_target = env!("BUILD_TARGET"),
        mcp_version = env!("CARGO_PKG_VERSION"),
        "instance started"
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let _tracing_guard = init_tracing(&config)?;
    log_instance_banner();
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
