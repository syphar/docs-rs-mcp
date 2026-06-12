use crate::{
    client::{dir_for_crate, download},
    config::Config,
};
use anyhow::{Context as _, Result};
use flate2::write::GzDecoder;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use tar::Archive;
use tokio::fs;
use tracing::debug;

/// build a crate source download url.
pub(crate) fn build_download_url(krate: &str, version: &str) -> String {
    format!("/crates/{krate}/{krate}-{version}.crate")
}

async fn fetch_crate(
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

async fn fetch_from_source(path: impl AsRef<Path>, find_path: &str) -> Result<Vec<u8>> {
    let path = path.as_ref();

    dbg!(&path);
    let tar_gz = File::open(path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    for entry in archive.entries()? {
        let entry = entry?;

        //     dbg!(&entry.path());
    }

    panic!();
    // archive.unpack(".")?;

    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixture, test_env};
    use test_case::test_case;

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
            .expect("expected docs to be present");

        assert!(path.exists());

        let cargo_toml = fetch_from_source(&path, "Cargo.toml").await?;

        // let file =

        // assert_eq!(docs.crate_version, Some(version.to_string()));

        // let root = &docs.paths[&docs.root];
        // assert_eq!(root.path, vec!["axum"]);
        // assert_eq!(root.kind, rustdoc_types::ItemKind::Module);

        Ok(())
    }
}
