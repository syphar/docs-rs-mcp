use crate::{client::get_source::fetch_cargo_manifest, context::Context};
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

/// Extract the local (non-inherited) value of a `MaybeInherited<T>`. Inherited
/// values reference a workspace's Cargo.toml which we don't fetch; treat them
/// as absent for the purposes of this tool.
fn local<T>(mi: Option<MaybeInherited<T>>) -> Option<T> {
    mi.and_then(MaybeInherited::as_local)
}

pub(crate) async fn crate_metadata(
    context: &Context,
    krate: &str,
    version: &semver::Version,
) -> Result<Option<CrateMetadata>> {
    let Some(manifest) = fetch_cargo_manifest(context, krate, version)
        .await?
        .clone()
        .as_ref()
    else {
        return Ok(None);
    };
    let Some(pkg) = &manifest.package else {
        return Ok(None);
    };

    let readme = local(pkg.readme.clone()).and_then(|s_or_b| match s_or_b {
        cargo_manifest::StringOrBool::String(s) => Some(s),
        cargo_manifest::StringOrBool::Bool(_) => None,
    });

    Ok(Some(CrateMetadata {
        name: pkg.name.clone(),
        version: local(pkg.version.clone()).unwrap_or_else(|| version.to_string()),
        description: local(pkg.description.clone()),
        repository: local(pkg.repository.clone()),
        homepage: local(pkg.homepage.clone()),
        documentation: local(pkg.documentation.clone()),
        license: local(pkg.license.clone()),
        license_file: local(pkg.license_file.clone()),
        readme,
        rust_version: local(pkg.rust_version.clone()),
        edition: local(pkg.edition.clone()).map(|e| e.as_str().to_string()),
        authors: local(pkg.authors.clone()).unwrap_or_default(),
        keywords: local(pkg.keywords.clone()).unwrap_or_default(),
        categories: local(pkg.categories.clone()).unwrap_or_default(),
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

        let meta = crate_metadata(env.context(), "axum", &version)
            .await?
            .expect("metadata present");
        assert_eq!(meta.name, "axum");
        assert_eq!(meta.version, "0.8.9");
        assert!(meta.description.is_some());
        assert!(meta.license.is_some());
        Ok(())
    }
}
