use anyhow::{Result, anyhow};
use directories::BaseDirs;
use reqwest::Url;
use std::{fs, path::PathBuf};

use crate::APP_NAME;

pub(crate) struct Config {
    pub(crate) cache_dir: PathBuf,
    pub(crate) docs_rs_server: Url,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self> {
        let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("can't find cache dir"))?;

        let cache_dir = base_dirs.cache_dir().join(APP_NAME);
        fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            docs_rs_server: Url::parse("https:://docs.rs")?,
        })
    }
}
