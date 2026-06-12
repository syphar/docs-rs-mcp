use crate::{
    client::get_docs::parse_rustdoc_json,
    context::{Config, Context},
};
use anyhow::{Result, bail};
use reqwest::Url;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) struct TestEnv {
    context: Context,
    pub(crate) server: mockito::ServerGuard,
    _cache_dir: tempfile::TempDir,
}

impl TestEnv {
    pub(crate) fn context(&self) -> &Context {
        &self.context
    }
}

pub(crate) async fn test_env() -> Result<TestEnv> {
    let server = mockito::Server::new_async().await;

    let cache_dir = tempfile::TempDir::new()?;
    let server_url = Url::parse(&server.url()).unwrap();
    let config = Config {
        cache_dir: cache_dir.path().to_path_buf(),
        docs_rs_server: server_url.clone(),
        static_crates_io: server_url.clone(),
        resolver_cache_ttl: Duration::from_secs(0),
    };

    Ok(TestEnv {
        context: Context::new(config),
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

    parse_rustdoc_json(&path).await
}
