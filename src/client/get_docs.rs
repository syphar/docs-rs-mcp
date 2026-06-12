use crate::{client::CLIENT, config::Config};
use anyhow::{Context as _, Result};
use futures_util::TryStreamExt;
use reqwest::{StatusCode, Url};
use std::{
    io,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    task::spawn_blocking,
};
use tokio_util::io::StreamReader;
use tracing::debug;

pub(crate) fn build_download_url(krate: &str, version: &str, target: &str) -> String {
    format!("/crate/{krate}/{version}/{target}/json.zst")
}

/// standard method for crates.io index to get the folder for a crate,
/// given a crate name.
fn dir_for_crate(output_path: &Path, name: &str) -> PathBuf {
    let mut path = output_path.to_owned();
    let name_lower = name.to_ascii_lowercase();
    match name_lower.len() {
        1 => path.push("1"),
        2 => path.push("2"),
        3 => path.extend(["3", &name_lower[..1]]),
        _ => path.extend([&name_lower[0..2], &name_lower[2..4]]),
    }
    path.push(name_lower);
    path
}

async fn fetch_rustdoc_json(
    config: &Config,
    krate: &str,
    version: &semver::Version,
    target: &str,
) -> Result<Option<PathBuf>> {
    let version = version.to_string();

    let target_dir = dir_for_crate(&config.cache_dir, krate);
    let target_path = target_dir
        .join(&version)
        .join(&target)
        .with_extension("json.zst");

    if fs::try_exists(&target_path).await? {
        debug!(target_path = %target_path.display(), "found rustdoc json");
        return Ok(Some(target_path));
    }

    fs::create_dir_all(&target_dir).await?;
    let url = config
        .docs_rs_server
        .join(&build_download_url(krate, &version, &target))
        .context("can't build download url")?;

    debug!(%url, target_path=%target_path.display(), "downloading rustdoc json");

    if !download(url, &target_path).await? {
        return Ok(None);
    }

    Ok(Some(target_path))
}

/// Returns `Ok(None)` when docs.rs has no build for this `(krate, version,
/// target)` triple (404). Callers may want to retry with a more common
/// target such as `x86_64-unknown-linux-gnu`.
pub(crate) async fn get_docs(
    config: &Config,
    krate: &str,
    version: &semver::Version,
    target: &str,
) -> Result<Option<rustdoc_types::Crate>> {
    let Some(path) = fetch_rustdoc_json(config, krate, version, target).await? else {
        return Ok(None);
    };

    let krate = spawn_blocking(move || -> Result<_, anyhow::Error> {
        let file = std::fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);
        let decoder = zstd::stream::read::Decoder::new(reader)?;

        Ok(serde_json::from_reader(decoder)?)
    })
    .await??;

    Ok(Some(krate))
}

/// `Ok(true)` on success, `Ok(false)` on 404. Other HTTP errors propagate.
async fn download(url: Url, target_path: &Path) -> Result<bool> {
    let response = CLIENT.get(url).send().await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(false);
    }
    let response = response.error_for_status()?;

    let stream = response.bytes_stream().map_err(io::Error::other);

    let mut reader = StreamReader::new(stream);
    let mut file = File::create(target_path).await?;

    tokio::io::copy(&mut reader, &mut file).await?;
    file.flush().await?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixture, test_env};

    const TEST_TARGET: &str = "x86_64-unknown-linux-gnu";

    #[tokio::test]
    async fn test_success() -> Result<()> {
        let mut env = test_env().await?;

        let version = semver::Version::new(0, 8, 9);
        let fixure_path = fixture("axum_0.8.9.json.zst")?;

        let _mock = env
            .server
            .mock(
                "GET",
                build_download_url("axum", &version.to_string(), TEST_TARGET).as_str(),
            )
            .with_status(200)
            .with_body_from_file(&fixure_path)
            .create();

        let docs = get_docs(env.config(), "axum", &version, TEST_TARGET)
            .await?
            .expect("expected docs to be present");
        assert_eq!(docs.crate_version, Some(version.to_string()));

        let root = &docs.paths[&docs.root];
        assert_eq!(root.path, vec!["axum"]);
        assert_eq!(root.kind, rustdoc_types::ItemKind::Module);

        Ok(())
    }
}
