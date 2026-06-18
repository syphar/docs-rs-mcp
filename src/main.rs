use crate::{
    context::{Config, Context},
    server::DocsServer,
};
use anyhow::{Context as _, Result};
use clap::Parser as _;
use directories::BaseDirs;
use figment::{
    Figment,
    providers::{Format as _, Serialized, Toml},
};
use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{SpanExporter, WithExportConfig as _};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use rmcp::{ServiceExt, transport::stdio};
use tokio::fs;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_appender::rolling;
use tracing_subscriber::{
    EnvFilter, Layer,
    filter::{FilterExt as _, filter_fn},
    fmt,
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
///   - (optional) OTLP export to `config.setup_opentelemetry`, INFO+ —
///     delivers spans to a collector (e.g. local Jaeger on `:4318`) for
///     waterfall visualization. Only attached when the endpoint is set.
///
/// All layers honour `RUST_LOG` if set.
///
/// Returns the OTLP tracer provider (when configured) so the caller can flush
/// and shut it down cleanly on exit.
fn init_tracing(config: &Config) -> Result<Option<SdkTracerProvider>> {
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
        .with_filter(
            EnvFilter::builder()
                .with_env_var(ENV_NAME)
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        );

    // Optional OTLP span export. `tracing_subscriber` implements `Layer` for
    // `Option<L>`, so a `None` here is simply a no-op layer.
    let (otel_layer, otel_provider) = match &config.opentelemetry_grpc_endpoint {
        Some(endpoint) => {
            // Use the OTLP gRPC (tonic) exporter rather than HTTP: the
            // `opentelemetry-http` + `reqwest` path establishes the TCP
            // connection but never sends the request body, failing every
            // export with a bogus "network error". tonic shares no code with
            // that path and integrates cleanly with our Tokio runtime.
            let exporter = SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint.as_str())
                .build()?;

            let provider = SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                // `pid` is a process-wide constant, so it belongs on the
                // Resource (attached to every exported span) rather than on a
                // single span — OTel span attributes are not inherited by
                // child spans, so a `pid` field on `serve` never reaches the
                // `tool.*` spans.
                .with_resource(
                    Resource::builder()
                        .with_service_name(APP_NAME)
                        .with_attribute(KeyValue::new("process.pid", i64::from(pid)))
                        .build(),
                )
                .build();

            let layer = tracing_opentelemetry::layer()
                .with_tracer(provider.tracer(APP_NAME))
                .with_filter(
                    EnvFilter::builder()
                        .with_env_var(ENV_NAME)
                        .with_default_directive(LevelFilter::INFO.into())
                        .from_env_lossy()
                        // rmcp's `serve_inner` span stays open for the whole
                        // process, so under batch export its `tool.*` children
                        // ship long before it does and appear parentless in
                        // Jaeger. Excluding it from OTLP export makes each
                        // `tool.*` span a root trace of its own.
                        .and(filter_fn(|meta| {
                            !(meta.is_span() && meta.name() == "serve_inner")
                        })),
                );

            (Some(layer), Some(provider))
        }
        None => (None, None),
    };

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .with(otel_layer)
        .init();

    Ok(otel_provider)
}

#[tokio::main]
async fn main() -> Result<()> {
    let config_file = BaseDirs::new()
        .context("can't find base dirs")?
        .config_dir()
        .join(APP_NAME)
        .join("config.toml");

    fs::create_dir_all(&config_file.parent().unwrap()).await?;

    let config: Config = Figment::new()
        .merge(Serialized::defaults(Config::try_parse()?))
        .merge(Toml::file(&config_file))
        .extract()?;

    let otel_provider = init_tracing(&config)?;

    info!(
        cwd = std::env::current_dir()
            .map(|cwd| cwd.display().to_string())
            .ok(),
        mcp_version = env!("CARGO_PKG_VERSION"),
        config_file = %config_file.display(),
        ?config,
        "instance started"
    );

    info!(log_dir = %config.log_dir.display(), "tracing initialized");

    let context = Context::new(config).await?;

    let service = DocsServer::new(context)
        .serve(stdio())
        .await
        .inspect_err(|e| {
            error!(?e, "serving error");
        })?;

    let result = service.waiting().await;

    // The client closing stdin returns us here; flush any buffered spans before
    // exit. On a hard SIGKILL this never runs, but the batch exporter will have
    // already shipped everything up to its last tick.
    if let Some(provider) = otel_provider
        && let Err(err) = provider.shutdown()
    {
        error!(?err, "opentelemetry shutdown error");
    }

    result?;
    Ok(())
}
