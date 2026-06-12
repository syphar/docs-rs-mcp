use anyhow::{Result, anyhow};
use directories::BaseDirs;
use moka::future::Cache;
use reqwest::Url;
use std::{fs, path::PathBuf, sync::Arc, time::Duration};

use crate::{APP_NAME, client::status::Status};

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct DocsKey {
    pub(crate) name: String,
    pub(crate) version: semver::Version,
    pub(crate) target: Option<String>,
}

pub(crate) struct Context {
    config: Config,
    pub(crate) resolver_cache: Cache<semver::VersionReq, Arc<Option<Status>>>,
    pub(crate) rustdoc_json_cache: Cache<DocsKey, Option<Arc<rustdoc_types::Crate>>>,
}

impl Context {
    pub(crate) fn new(config: Config) -> Self {
        Self {
            resolver_cache: Cache::builder()
                // cache for 1h
                .time_to_live(config.resolver_cache_ttl)
                .build(),
            rustdoc_json_cache: Cache::builder().build(),
            config,
        }
    }
    pub(crate) fn config(&self) -> &Config {
        &self.config
    }
}

pub(crate) struct Config {
    pub(crate) cache_dir: PathBuf,
    pub(crate) docs_rs_server: Url,
    pub(crate) static_crates_io: Url,
    pub(crate) resolver_cache_ttl: Duration,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self> {
        let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("can't find cache dir"))?;

        let cache_dir = base_dirs.cache_dir().join(APP_NAME);
        fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            docs_rs_server: Url::parse("https://docs.rs")?,
            static_crates_io: Url::parse("https://static.crates.io")?,
            resolver_cache_ttl: Duration::from_secs(60 * 60),
        })
    }
}
