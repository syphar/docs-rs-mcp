pub(crate) mod changelog;
pub(crate) mod crate_metadata;
pub(crate) mod dependency_tree;
pub(crate) mod find_examples;
pub(crate) mod get_docs;
pub(crate) mod get_item;
pub(crate) mod get_source;
pub(crate) mod inspect_feature_flags;
pub(crate) mod list_implementors;
pub(crate) mod list_impls;
pub(crate) mod list_methods;
pub(crate) mod list_module;
pub(crate) mod search_items;
pub(crate) mod status;

use anyhow::Result;
use futures_util::TryStreamExt as _;
use reqwest::{StatusCode, Url};
use std::{
    io,
    path::{Path, PathBuf},
    sync::LazyLock,
};
use tokio::{fs::File, io::AsyncWriteExt as _};
use tokio_util::io::StreamReader;

const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));

pub(crate) static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
        .expect("can't create request client & connection pool")
});

/// standard method for crates.io index to get the folder for a crate,
/// given a crate name.
fn dir_for_crate(output_path: &Path, name: &str, version: &str) -> PathBuf {
    let mut path = output_path.to_owned();
    let name_lower = name.to_ascii_lowercase();
    match name_lower.len() {
        1 => path.push("1"),
        2 => path.push("2"),
        3 => path.extend(["3", &name_lower[..1]]),
        _ => path.extend([&name_lower[0..2], &name_lower[2..4]]),
    }
    path.push(name_lower);
    path.push(version);
    path
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
