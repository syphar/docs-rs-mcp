use crate::{
    client::{CLIENT, status::get_docs_status},
    context::Context,
    errors::Error,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct VersionsResponse {
    versions: Vec<ApiVersion>,
}

#[derive(Debug, Deserialize)]
struct ApiVersion {
    num: semver::Version,
    yanked: bool,
    created_at: String,
    rust_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct VersionInfo {
    pub(crate) version: semver::Version,
    pub(crate) yanked: bool,
    pub(crate) prerelease: bool,
    pub(crate) published_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rust_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) doc_status: Option<bool>,
}

pub(crate) async fn list_versions(
    context: &Context,
    krate: &str,
    limit: usize,
    include_yanked: bool,
    include_prerelease: bool,
    check_docs: bool,
) -> Result<Vec<VersionInfo>, Error> {
    let url = context
        .config()
        .crates_io_server
        .join(&format!("/api/v1/crates/{krate}/versions?per_page=100"))
        .map_err(anyhow::Error::from)?;
    let response: VersionsResponse = CLIENT
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut out = Vec::new();
    for version in response.versions {
        let prerelease = !version.num.pre.is_empty();
        if (!include_yanked && version.yanked) || (!include_prerelease && prerelease) {
            continue;
        }
        let doc_status = if check_docs {
            let req = semver::VersionReq::parse(&format!("={}", version.num))
                .map_err(anyhow::Error::from)?;
            get_docs_status(context, krate, &req)
                .await?
                .map(|status| status.doc_status)
        } else {
            None
        };
        out.push(VersionInfo {
            version: version.num,
            yanked: version.yanked,
            prerelease,
            published_at: version.created_at,
            rust_version: version.rust_version,
            doc_status,
        });
        if out.len() == limit {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;
    use anyhow::Result;

    #[tokio::test]
    async fn filters_yanked_and_prerelease_versions() -> Result<()> {
        let mut env = test_env().await?;
        let _mock = env
            .server
            .mock("GET", "/api/v1/crates/demo/versions?per_page=100")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"versions":[
                    {"num":"2.0.0-alpha.1","yanked":false,"created_at":"2026-01-03","rust_version":null},
                    {"num":"1.1.0","yanked":true,"created_at":"2026-01-02","rust_version":"1.80"},
                    {"num":"1.0.0","yanked":false,"created_at":"2026-01-01","rust_version":"1.75"}
                ]}"#,
            )
            .create();

        let versions = list_versions(env.context(), "demo", 20, false, false, false).await?;
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, semver::Version::new(1, 0, 0));
        Ok(())
    }
}
