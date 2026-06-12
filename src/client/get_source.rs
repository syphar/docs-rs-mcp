use crate::{
    client::{dir_for_crate, download},
    config::Config,
};
use anyhow::{Context as _, Result, bail};
use flate2::read::GzDecoder;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use tar::Archive;
use tokio::{fs, task::spawn_blocking};
use tracing::debug;

/// build a crate source download url.
pub(crate) fn build_download_url(krate: &str, version: &str) -> String {
    format!("/crates/{krate}/{krate}-{version}.crate")
}

pub(crate) async fn fetch_crate(
    config: &Config,
    krate: &str,
    version: &semver::Version,
) -> Result<Option<PathBuf>> {
    let version = version.to_string();

    let target_dir = dir_for_crate(&config.cache_dir, krate, &version);
    let target_path = target_dir.join("source").with_extension("crate");

    if fs::try_exists(&target_path).await? {
        debug!(target_path = %target_path.display(), "found crate source");
        return Ok(Some(target_path));
    }

    fs::create_dir_all(&target_dir).await?;
    let url = config
        .static_crates_io
        .join(&build_download_url(krate, &version))
        .context("can't build download url")?;

    debug!(%url, target_path=%target_path.display(), "downloading crate source");

    if !download(url, &target_path).await? {
        return Ok(None);
    }

    Ok(Some(target_path))
}

pub(crate) async fn extract_source(
    path: impl AsRef<Path>,
    name: &str,
    version: &str,
) -> Result<PathBuf> {
    let path = path.as_ref().to_path_buf();
    let output_dir = path.parent().unwrap().join("extracted");
    let source_path = output_dir.join(format!("{name}-{version}"));

    spawn_blocking(move || -> Result<PathBuf> {
        std::fs::create_dir_all(&output_dir)?;

        let tar_gz = File::open(&path)
            .with_context(|| format!("opening crate archive {}", path.display()))?;
        let mut archive = Archive::new(GzDecoder::new(tar_gz));
        archive.unpack(&output_dir)?;

        if !source_path.is_dir() {
            bail!(
                "broken crate archive, missing source directory {:?}",
                source_path
            );
        };

        Ok(source_path)
    })
    .await?
}

pub(crate) async fn fetch_source(
    config: &Config,
    krate: &str,
    version: &semver::Version,
) -> Result<Option<PathBuf>> {
    let Some(crate_file) = fetch_crate(config, krate, version).await? else {
        return Ok(None);
    };

    let version_str = version.to_string();
    let source_dir = extract_source(&crate_file, krate, &version_str).await?;

    Ok(Some(source_dir))
}

/// Convenience: fetch the crate archive (if needed), read `Cargo.toml`, and
/// parse it into the typed `cargo_manifest::Manifest`. Pure parser — does
/// not shell out to `cargo`. Returns `Ok(None)` when the crate/version isn't
/// on crates.io.
pub(crate) async fn fetch_cargo_manifest(
    config: &Config,
    krate: &str,
    version: &semver::Version,
) -> Result<Option<cargo_manifest::Manifest>> {
    let Some(source_dir) = fetch_source(config, krate, version).await? else {
        return Ok(None);
    };

    let cargo_toml = source_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(None);
    }

    let bytes = tokio::fs::read(&cargo_toml).await?;
    let manifest = cargo_manifest::Manifest::from_slice(&bytes).context("parsing Cargo.toml")?;
    Ok(Some(manifest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixture, test_env};

    #[tokio::test]
    async fn test_success() -> Result<()> {
        let mut env = test_env().await?;

        let version = semver::Version::new(0, 8, 9);
        let fixure_path = fixture("axum-0.8.9.crate")?;

        let _mock = env
            .server
            .mock(
                "GET",
                build_download_url("axum", &version.to_string()).as_str(),
            )
            .with_status(200)
            .with_body_from_file(&fixure_path)
            .create();

        let path = fetch_source(env.config(), "axum", &version)
            .await?
            .expect("expected crate source to be present");

        assert!(path.exists());

        let cargo_toml = path.join("Cargo.toml");
        assert!(cargo_toml.exists());

        let cargo_toml = tokio::fs::read_to_string(cargo_toml).await?;
        assert!(cargo_toml.contains("name = \"axum\""));
        assert!(cargo_toml.contains("version = \"0.8.9\""));

        Ok(())
    }
}
