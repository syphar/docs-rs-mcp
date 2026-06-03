use crate::config::Config;
use anyhow::Result;
use async_compression::tokio::bufread::ZstdDecoder;
use futures_util::TryStreamExt;
use reqwest::Url;
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

fn build_download_url(krate: &str, req_version: &str) -> Result<Url> {
    Ok(Url::parse(&format!(
        "https://docs.rs/crate/{krate}/{req_version}/json.zst"
    ))?)
}

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
) -> Result<PathBuf> {
    let version = version.to_string();

    let target_dir = dir_for_crate(&config.cache_dir, krate);
    let target_path = target_dir.join(&version).with_extension("json");

    if fs::try_exists(&target_path).await? {
        debug!(target_path = %target_path.display(), "found rustdoc json");
        return Ok(target_path);
    }

    fs::create_dir_all(&target_dir).await?;
    let url = build_download_url(krate, &version)?;

    debug!(%url, target_path=%target_path.display(), "downloading rustdoc json");

    download_zstd_to_file(url, &target_path).await?;

    Ok(target_path)
}

pub(crate) async fn get_docs(
    config: &Config,
    krate: &str,
    version: &semver::Version,
) -> Result<rustdoc_types::Crate> {
    let path = fetch_rustdoc_json(config, krate, version).await?;

    let krate = spawn_blocking(move || {
        let file = std::fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);

        Ok::<_, anyhow::Error>(serde_json::from_reader(reader)?)
    })
    .await??;

    Ok(krate)
}

async fn download_zstd_to_file(url: Url, target_path: &Path) -> Result<()> {
    let response = reqwest::get(url).await?.error_for_status()?;

    let stream = response
        .bytes_stream()
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err));

    let reader = StreamReader::new(stream);
    let mut decoder = ZstdDecoder::new(reader);
    let mut file = File::create(target_path).await?;

    tokio::io::copy(&mut decoder, &mut file).await?;
    file.flush().await?;

    Ok(())
}
