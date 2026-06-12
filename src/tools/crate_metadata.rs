use crate::{client::crate_metadata, config::Config, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct CrateMetadataArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
}

pub(crate) async fn handle(
    config: &Config,
    args: CrateMetadataArgs,
) -> Result<CallToolResult, McpError> {
    let meta = crate_metadata::crate_metadata(config, &args.krate, args.version.as_ref())
        .await
        .map_err(|err| McpError::internal_error(err.to_string(), None))?
        .ok_or_else(|| {
            McpError::resource_not_found("crate or version not found on crates.io", None)
        })?;

    Ok(CallToolResult::structured(
        serde_json::to_value(meta)
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
