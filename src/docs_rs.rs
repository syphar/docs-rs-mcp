use crate::client::CLIENT;
use anyhow::Result;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Status {
    pub(crate) doc_status: bool,
    pub(crate) version: semver::Version,
}

pub(crate) async fn get_docs_status(krate: &str, req_version: &str) -> Result<Option<Status>> {
    let response = CLIENT
        .get(&format!(
            "https://docs.rs/crate/{krate}/{req_version}/status.json"
        ))
        .send()
        .await?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    Ok(Some(response.error_for_status()?.json().await?))
}
