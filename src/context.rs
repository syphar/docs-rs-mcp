use crate::{
    APP_NAME,
    client::{get_docs::LoadedDocs, status::Status},
};
use anyhow::Result;
use clap::Parser;
use directories::BaseDirs;
use moka::future::Cache;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_with::{DurationSeconds, serde_as};
use std::{fmt, num, path::PathBuf, sync::Arc, time::Duration};
use tokio::fs;

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct DocsKey {
    pub(crate) name: String,
    pub(crate) version: semver::Version,
    pub(crate) target: Option<String>,
}

pub(crate) struct Context {
    config: Config,
    pub(crate) resolver_cache: Cache<(String, semver::VersionReq), Arc<Option<Status>>>,
    pub(crate) rustdoc_json_cache: Cache<DocsKey, Arc<LoadedDocs>>,
    pub(crate) cargo_manifest_cache: Cache<DocsKey, Arc<cargo_manifest::Manifest>>,
}

impl Context {
    pub(crate) async fn new(config: Config) -> Result<Self> {
        fs::create_dir_all(&config.cache_dir).await?;
        fs::create_dir_all(&config.log_dir).await?;

        Ok(Self {
            resolver_cache: Cache::builder()
                .time_to_live(config.resolver_cache_ttl)
                .build(),
            rustdoc_json_cache: Cache::builder().build(),
            cargo_manifest_cache: Cache::builder().build(),
            config,
        })
    }
    pub(crate) fn config(&self) -> &Config {
        &self.config
    }
}

fn parse_seconds(value: &str) -> Result<Duration, num::ParseIntError> {
    value.parse::<u64>().map(Duration::from_secs)
}

/// Command-line arguments. Every option is optional so that an unset flag
/// falls through to the config file, the environment, or the built-in default
/// (in that order of precedence). The flag names line up 1:1 with the
/// [`Config`] fields so clap's output can be merged straight into figment.
#[serde_as]
#[derive(Parser, Serialize, Deserialize)]
#[command(name = APP_NAME, version, about)]
pub(crate) struct Config {
    /// Directory for cached rustdoc JSON and manifests.
    #[arg(
        long,
        value_name = "DIR",
        default_value_os_t = BaseDirs::new().expect("can't find base dirs").cache_dir().join(APP_NAME)
    )]
    pub(crate) cache_dir: PathBuf,

    /// Directory for rolling log files.
    #[arg(
        long,
        value_name = "DIR",
        default_value_os_t = BaseDirs::new().expect("can't find base dirs").cache_dir().join(APP_NAME).join("_logs"),
    )]
    pub(crate) log_dir: PathBuf,

    /// Base URL of the docs.rs server.
    #[arg(
        long,
        value_name = "URL",
        default_value_t = Url::parse("https://docs.rs").unwrap()
    )]
    pub(crate) docs_rs_server: Url,

    /// Base URL of the static crates.io CDN.
    #[arg(
        long,
        value_name = "URL",
        default_value_t = Url::parse("https://static.crates.io").unwrap()
    )]
    pub(crate) static_crates_io: Url,

    /// How long (in seconds) to cache version resolution results.
    #[arg(
        long,
        value_name = "SECONDS",
        value_parser = parse_seconds,
        default_value = "3600"
    )]
    #[serde_as(as = "DurationSeconds<u64>")]
    pub(crate) resolver_cache_ttl: Duration,

    /// OTLP gRPC endpoint for OpenTelemetry span export (omit to disable).
    #[arg(long, value_name = "URL")]
    pub(crate) opentelemetry_grpc_endpoint: Option<Url>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("cache_dir", &self.cache_dir.display())
            .field("log_dir", &self.log_dir.display())
            .field("docs_rs_server", &self.docs_rs_server.to_string())
            .field("static_crates_io", &self.static_crates_io.to_string())
            .field("resolver_cache_ttl", &self.resolver_cache_ttl.as_secs())
            .field(
                "opentelemetry_grpc_endpoint",
                &self
                    .opentelemetry_grpc_endpoint
                    .as_ref()
                    .map(|u| u.to_string()),
            )
            .finish()
    }
}
