use crate::config::Config;
use anyhow::Result;
use reqwest::Url;

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
