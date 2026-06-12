use crate::{
    client::{dir_for_crate, download},
    config::Config,
};
use anyhow::{Context as _, Result, bail};
use flate2::read::GzDecoder;
use std::{
    fs::File,
    io::Read as _,
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

/// Read one file out of a `.crate` archive (gzipped tar). `find_path` is
/// matched against the archive entry path *with the top-level
/// `<krate>-<version>/` prefix stripped*, so pass e.g. `"Cargo.toml"` or
/// `"examples/hello.rs"`.
pub(crate) async fn fetch_from_source<P>(
    path: impl AsRef<Path>,
    find_paths: impl IntoIterator<Item = P>,
) -> Result<Option<(PathBuf, Vec<u8>)>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref().to_path_buf();
    let find_paths: Vec<_> = find_paths
        .into_iter()
        .map(|p| p.as_ref().to_path_buf())
        .collect();

    spawn_blocking(move || -> Result<_> {
        let tar_gz = File::open(&path)
            .with_context(|| format!("opening crate archive {}", path.display()))?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let entry_path = entry.path()?.to_path_buf();
            // Strip the leading `<krate>-<version>/` component.
            let mut components = entry_path.components();
            let _root = components.next();
            let relative: PathBuf = components.collect();

            for find_path in &find_paths {
                if relative == find_path.as_path() {
                    let mut buf = Vec::with_capacity(entry.size() as usize);
                    entry.read_to_end(&mut buf)?;
                    return Ok(Some((find_path.to_path_buf(), buf)));
                }
            }
        }
        Ok(None)
    })
    .await?
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

/// Convenience: fetch the crate archive (if needed), read `Cargo.toml`, and
/// parse it. Returns `Ok(None)` when the crate/version isn't on crates.io.
pub(crate) async fn fetch_cargo_toml(
    config: &Config,
    krate: &str,
    version: &semver::Version,
) -> Result<Option<toml::Value>> {
    let Some(archive_path) = fetch_crate(config, krate, version).await? else {
        return Ok(None);
    };
    let Some((_, bytes)) = fetch_from_source(&archive_path, ["Cargo.toml"]).await? else {
        return Ok(None);
    };
    let text = std::str::from_utf8(&bytes).context("Cargo.toml is not valid UTF-8")?;
    let value: toml::Value = toml::from_str(text).context("parsing Cargo.toml")?;
    Ok(Some(value))
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

        let path = fetch_crate(env.config(), "axum", &version)
            .await?
            .expect("expected crate source to be present");

        assert!(path.exists());

        let (_, cargo_toml) = fetch_from_source(&path, ["Cargo.toml"])
            .await?
            .expect("should exist");

        let cargo_toml = str::from_utf8(&cargo_toml)?;
        assert!(cargo_toml.contains("name = \"axum\""));
        assert!(cargo_toml.contains("version = \"0.8.9\""));

        Ok(())
    }
}
