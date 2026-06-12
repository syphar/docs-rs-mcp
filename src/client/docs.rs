use crate::{client::CLIENT, config::Config, types::rustdoc_types::ItemKind};
use anyhow::Result;
use async_compression::tokio::bufread::ZstdDecoder;
use futures_util::TryStreamExt;
use reqwest::Url;
use rustdoc_types::Id;
use serde::Serialize;
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

pub(crate) fn build_download_url(krate: &str, version: &str) -> String {
    format!("/crate/{krate}/{version}/json.zst")
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
) -> Result<PathBuf> {
    let version = version.to_string();

    let target_dir = dir_for_crate(&config.cache_dir, krate);
    let target_path = target_dir.join(&version).with_extension("json");

    if fs::try_exists(&target_path).await? {
        debug!(target_path = %target_path.display(), "found rustdoc json");
        return Ok(target_path);
    }

    fs::create_dir_all(&target_dir).await?;
    let url = config
        .docs_rs_server
        .join(&build_download_url(krate, &version))?;

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

#[derive(Debug, Serialize)]
pub(crate) struct SearchItemMatch {
    pub(crate) id: Id,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) kind: ItemKind,
}

pub(crate) fn search_items(
    docs: &rustdoc_types::Crate,
    query: &str,
    kind_filter: Option<ItemKind>,
    limit: usize,
) -> Vec<SearchItemMatch> {
    let query = query.to_lowercase();

    let mut matches = docs
        .index
        .values()
        .filter_map(|item| {
            let kind: ItemKind = item.inner.item_kind().into();
            if kind_filter.is_some_and(|filter| filter != kind) {
                return None;
            }

            let path = docs
                .paths
                .get(&item.id)
                .map(|summary| summary.path.join("::"))
                .or_else(|| item.name.clone())?;
            let name = item.name.clone().unwrap_or_default();
            let haystack = format!("{name} {path}").to_lowercase();

            haystack.contains(&query).then_some(SearchItemMatch {
                id: item.id,
                name,
                path,
                kind,
            })
        })
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });
    matches.truncate(limit);
    matches
}

async fn download_zstd_to_file(url: Url, target_path: &Path) -> Result<()> {
    let response = CLIENT.get(url).send().await?.error_for_status()?;

    let stream = response.bytes_stream().map_err(io::Error::other);

    let reader = StreamReader::new(stream);
    let mut decoder = ZstdDecoder::new(reader);
    let mut file = File::create(target_path).await?;

    tokio::io::copy(&mut decoder, &mut file).await?;
    file.flush().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixture, test_env};

    #[tokio::test]
    async fn test_success() -> Result<()> {
        let mut env = test_env().await?;

        let version = semver::Version::new(0, 8, 9);
        let fixure_path = fixture("axum_0.8.9.json.zst")?;

        let _mock = env
            .server
            .mock(
                "GET",
                build_download_url("axum", &version.to_string()).as_str(),
            )
            .with_status(200)
            .with_body_from_file(&fixure_path)
            .create();

        let docs = get_docs(env.config(), "axum", &version).await?;
        assert_eq!(docs.crate_version, Some(version.to_string()));

        let root = &docs.paths[&docs.root];
        assert_eq!(root.path, vec!["axum"]);
        assert_eq!(root.kind, rustdoc_types::ItemKind::Module);

        Ok(())
    }
}
