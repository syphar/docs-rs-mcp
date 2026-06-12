use crate::{client::changelog, context::Config, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct ChangelogArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version (which release's archive to fetch). Use
    /// `resolve_version` first if you only have a semver requirement.
    pub(crate) version: Version,
    /// Optional version string to extract a specific section for (best-effort:
    /// matches a markdown heading containing this string). Omit to return the
    /// full changelog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) section_version: Option<String>,
}

pub(crate) async fn handle(
    config: &Config,
    args: ChangelogArgs,
) -> Result<CallToolResult, McpError> {
    let cl = changelog::changelog(
        config,
        &args.krate,
        args.version.as_ref(),
        args.section_version.as_deref(),
    )
    .await
    .map_err(|err| McpError::internal_error(err.to_string(), None))?
    .ok_or_else(|| {
        McpError::resource_not_found("no changelog file found in this crate's source", None)
    })?;

    Ok(CallToolResult::structured(
        serde_json::to_value(cl).map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
