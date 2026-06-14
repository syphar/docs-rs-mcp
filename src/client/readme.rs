use crate::{
    client::{
        crate_metadata::local,
        get_source::{fetch_source, parse_cargo_manifest},
    },
    context::Context,
};
use anyhow::Result;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use tokio::fs;

#[derive(Debug, Serialize)]
pub(crate) struct Readme {
    /// Path of the README file inside the extracted crate source.
    pub(crate) source_file: String,
    /// README contents, decoded as UTF-8 with replacement for invalid bytes.
    pub(crate) content: String,
}

const DEFAULT_README_CANDIDATES: &[&str] = &["README.md", "README.txt", "README"];

fn safe_readme_path(source_dir: &Path, readme_path: impl AsRef<Path>) -> Option<PathBuf> {
    let readme_path = readme_path.as_ref();

    if readme_path.is_absolute()
        || readme_path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }

    Some(source_dir.join(readme_path))
}

async fn default_readme(source_dir: &Path) -> Result<Option<PathBuf>> {
    for candidate in DEFAULT_README_CANDIDATES {
        let path = source_dir.join(candidate);
        if fs::try_exists(&path).await? {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

async fn manifest_readme(source_dir: &Path) -> Result<Option<PathBuf>> {
    let Some(manifest) = parse_cargo_manifest(source_dir).await? else {
        return default_readme(source_dir).await;
    };
    let Some(pkg) = manifest.package.as_ref() else {
        return default_readme(source_dir).await;
    };

    match local(&pkg.readme) {
        Some(cargo_manifest::StringOrBool::String(path)) => {
            let Some(path) = safe_readme_path(source_dir, &path) else {
                return Ok(None);
            };
            Ok(fs::try_exists(&path).await?.then_some(path))
        }
        Some(cargo_manifest::StringOrBool::Bool(false)) => Ok(None),
        Some(cargo_manifest::StringOrBool::Bool(true)) | None => default_readme(source_dir).await,
    }
}

pub(crate) async fn readme(
    context: &Context,
    krate: &str,
    version: &semver::Version,
) -> Result<Option<Readme>> {
    let Some(source_dir) = fetch_source(context, krate, version).await? else {
        return Ok(None);
    };
    let Some(readme_path) = manifest_readme(&source_dir).await? else {
        return Ok(None);
    };

    let bytes = fs::read(&readme_path).await?;
    let content = String::from_utf8_lossy(&bytes).into_owned();

    Ok(Some(Readme {
        source_file: readme_path.display().to_string(),
        content,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_env;

    #[tokio::test]
    async fn test_axum_readme() -> Result<()> {
        let mut env = test_env().await?;
        let version = semver::Version::new(0, 8, 9);
        let fixture = crate::test_utils::fixture("axum-0.8.9.crate")?;
        let _mock = env
            .server
            .mock("GET", "/crates/axum/axum-0.8.9.crate")
            .with_status(200)
            .with_body_from_file(&fixture)
            .create();

        let readme = readme(env.context(), "axum", &version)
            .await?
            .expect("README present");

        assert!(readme.source_file.ends_with("/README.md"));
        assert!(readme.content.contains("axum"));
        Ok(())
    }
}
