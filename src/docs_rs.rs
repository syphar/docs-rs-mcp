use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Status {
    pub(crate) doc_status: bool,
    pub(crate) version: semver::Version,
}

pub(crate) async fn get_docs_status(krate: &str, req_version: &str) -> Result<Status> {
    Ok(reqwest::get(&format!(
        "https://docs.rs/crate/{krate}/{req_version}/status.json"
    ))
    .await?
    .error_for_status()?
    .json()
    .await?)
}
