use crate::{client::CLIENT, context::Context};
use anyhow::Result;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub(crate) struct Status {
    pub(crate) doc_status: bool,
    pub(crate) version: semver::Version,
}

pub(crate) async fn get_docs_status(
    context: &Context,
    krate: &str,
    req_version: impl Into<&semver::VersionReq>,
) -> Result<Option<Status>> {
    let req_version = req_version.into();

    let response = CLIENT
        .get(
            context
                .config()
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
    use crate::test_utils::test_env;

    #[tokio::test]
    async fn test_success() -> Result<()> {
        let mut env = test_env().await?;

        let version = semver::Version::new(1, 2, 3);
        let status = Status {
            doc_status: true,
            version,
        };

        let _mock = env
            .server
            .mock("GET", "/crate/itertools/^1.2.3/status.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&status)?)
            .create();

        assert_eq!(
            get_docs_status(
                env.context(),
                "itertools",
                &semver::VersionReq::parse("1.2.3").unwrap(),
            )
            .await?,
            Some(status)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_not_found() -> Result<()> {
        let mut env = test_env().await?;

        let _mock = env
            .server
            .mock("GET", "/crate/itertools/^1.2.3/status.json")
            .with_status(404)
            .create();

        assert!(
            get_docs_status(
                env.context(),
                "itertools",
                &semver::VersionReq::parse("1.2.3").unwrap(),
            )
            .await?
            .is_none(),
        );

        Ok(())
    }
}
