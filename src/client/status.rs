use crate::{client::CLIENT, config::Config};
use anyhow::Result;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub(crate) struct Status {
    pub(crate) doc_status: bool,
    pub(crate) version: semver::Version,
}

pub(crate) async fn get_docs_status(
    config: &Config,
    krate: &str,
    req_version: &semver::VersionReq,
) -> Result<Option<Status>> {
    let response = CLIENT
        .get(
            config
                .docs_rs_server
                .join(&format!("/crate/{krate}/{req_version}/status.json"))?,
        )
        .send()
        .await?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    Ok(Some(response.error_for_status()?.json().await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semver_types::VersionReq;
    use reqwest::Url;

    #[tokio::test]
    async fn test_success() -> Result<()> {
        let mut server = mockito::Server::new_async().await;

        let version = semver::Version::new(1, 2, 3);
        let status = Status {
            doc_status: true,
            version,
        };

        let _mock = server
            .mock("GET", "/crate/itertools/^1.2.3/status.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&status)?)
            .create();

        let config = Config {
            cache_dir: tempfile::tempdir()?.keep(),
            docs_rs_server: Url::parse(&server.url()).unwrap(),
        };

        assert_eq!(
            get_docs_status(&config, "itertools", &VersionReq::parse("1.2.3").unwrap()).await?,
            Some(status)
        );

        Ok(())
    }
}
