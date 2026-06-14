use crate::{client::get_source::fetch_cargo_manifest, context::Context, errors::Error};
use anyhow::Result;
use cargo_manifest::MaybeInherited;
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

/// Extract the local (non-inherited) value of a `MaybeInherited<T>`, cloning
/// it out of the borrowed manifest. Inherited values reference a workspace's
/// Cargo.toml which we don't fetch; treat them as absent for this tool.
pub(crate) fn local<T: Clone>(mi: &Option<MaybeInherited<T>>) -> Option<T> {
    mi.as_ref().and_then(|m| m.as_ref().as_local()).cloned()
}

pub(crate) async fn crate_metadata(
    context: &Context,
    krate: &str,
    version: &semver::Version,
) -> Result<CrateMetadata, Error> {
    let manifest = fetch_cargo_manifest(context, krate, version).await?;
    let Some(pkg) = manifest.package.as_ref() else {
        return Err(Error::MissingMetadata("missing package metadata".into()));
    };

    let readme = local(&pkg.readme).and_then(|s_or_b| match s_or_b {
        cargo_manifest::StringOrBool::String(s) => Some(s),
        cargo_manifest::StringOrBool::Bool(_) => None,
    });

    Ok(CrateMetadata {
        name: pkg.name.clone(),
        version: local(&pkg.version).unwrap_or_else(|| version.to_string()),
        description: local(&pkg.description),
        repository: local(&pkg.repository),
        homepage: local(&pkg.homepage),
        documentation: local(&pkg.documentation),
        license: local(&pkg.license),
        license_file: local(&pkg.license_file),
        readme,
        rust_version: local(&pkg.rust_version),
        edition: local(&pkg.edition).map(|e| e.as_str().to_string()),
        authors: local(&pkg.authors).unwrap_or_default(),
        keywords: local(&pkg.keywords).unwrap_or_default(),
        categories: local(&pkg.categories).unwrap_or_default(),
    })
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

        let meta = crate_metadata(env.context(), "axum", &version).await?;
        assert_eq!(meta.name, "axum");
        assert_eq!(meta.version, "0.8.9");
        assert!(meta.description.is_some());
        assert!(meta.license.is_some());
        Ok(())
    }
}
