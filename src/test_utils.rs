use crate::{client::get_docs::parse_rustdoc_json, config::Config};
use anyhow::{Result, bail};
use reqwest::Url;
use std::path::{Path, PathBuf};
use tokio::task::spawn_blocking;

pub(crate) struct TestEnv {
    config: Config,
    pub(crate) server: mockito::ServerGuard,
    _cache_dir: tempfile::TempDir,
}

impl TestEnv {
    pub(crate) fn config(&self) -> &Config {
        &self.config
    }
}

pub(crate) async fn test_env() -> Result<TestEnv> {
    let server = mockito::Server::new_async().await;

    let cache_dir = tempfile::TempDir::new()?;
    let config = Config {
        cache_dir: cache_dir.path().to_path_buf(),
        docs_rs_server: Url::parse(&server.url()).unwrap(),
    };

    Ok(TestEnv {
        config,
        server,
        _cache_dir: cache_dir,
    })
}

pub(crate) fn fixture(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/")
        .join(path.as_ref());

    if !path.exists() {
        bail!("fixture {} doesn't exist", path.display());
    } else {
        Ok(path)
    }
}

pub(crate) async fn docs_fixture(path: impl AsRef<Path>) -> Result<rustdoc_types::Crate> {
    let path = fixture(path)?;

    let krate = spawn_blocking(move || parse_rustdoc_json(&path)).await??;

    Ok(krate)
}
