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
};
use tokio_util::io::StreamReader;

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

pub(crate) async fn fetch_rustdoc_json(
    config: &Config,
    krate: &str,
    req_version: Option<&str>,
) -> Result<PathBuf> {
    let req_version = req_version.unwrap_or("latest");

    let target_dir = dir_for_crate(&config.cache_dir, krate);
    let target_path = target_dir.join(req_version).with_extension("json");

    if fs::try_exists(&target_path).await? {
        return Ok(target_path);
    }

    fs::create_dir_all(&target_dir).await?;
    let url = build_download_url(krate, req_version)?;

    download_zstd_to_file(url, &target_path).await?;

    Ok(target_path)
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
