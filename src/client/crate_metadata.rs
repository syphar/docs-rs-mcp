use crate::{client::get_source::fetch_cargo_toml, config::Config};
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct CrateMetadata {
    pub(crate) name: String,
    pub(crate) version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) license_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) readme: Option<String>,
    /// Minimum supported Rust version from `package.rust-version`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rust_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) edition: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) authors: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) keywords: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) categories: Vec<String>,
}

pub(crate) async fn crate_metadata(
    config: &Config,
    krate: &str,
    version: &semver::Version,
) -> Result<Option<CrateMetadata>> {
    let Some(cargo) = fetch_cargo_toml(config, krate, version).await? else {
        return Ok(None);
    };
    let pkg = cargo.get("package").and_then(|p| p.as_table());
    let Some(pkg) = pkg else { return Ok(None) };

    let str_opt = |key: &str| pkg.get(key).and_then(|v| v.as_str()).map(str::to_string);
    let str_vec = |key: &str| {
        pkg.get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };

    Ok(Some(CrateMetadata {
        name: str_opt("name").unwrap_or_else(|| krate.to_string()),
        version: str_opt("version").unwrap_or_else(|| version.to_string()),
        description: str_opt("description"),
        repository: str_opt("repository"),
        homepage: str_opt("homepage"),
        documentation: str_opt("documentation"),
        license: str_opt("license"),
        license_file: str_opt("license-file"),
        readme: str_opt("readme"),
        rust_version: str_opt("rust-version"),
        edition: str_opt("edition"),
        authors: str_vec("authors"),
        keywords: str_vec("keywords"),
        categories: str_vec("categories"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    #[tokio::test]
    async fn test_axum_metadata() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = crate::test_utils::fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        let meta = crate_metadata(env.config(), "axum", &version)
            .await?
            .expect("metadata present");
        assert_eq!(meta.name, "axum");
        assert_eq!(meta.version, "0.8.9");
        assert!(meta.description.is_some());
        assert!(meta.license.is_some());
        Ok(())
    }
}
