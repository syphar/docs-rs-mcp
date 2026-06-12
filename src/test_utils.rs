use crate::config::Config;
use anyhow::{Result, bail};
use async_compression::tokio::bufread::ZstdDecoder;
use reqwest::Url;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt as _;

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
    use tokio::{fs, io};

    let path = fixture(path)?;

    let file = fs::File::open(path).await?;
    let reader = io::BufReader::new(file);
    let mut decoder = ZstdDecoder::new(reader);

    let mut out = Vec::new();
    decoder.read_to_end(&mut out).await?;

    Ok(serde_json::from_slice(&out)?)
}
