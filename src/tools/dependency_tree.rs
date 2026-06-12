use crate::{client::dependency_tree, config::Config, types::semver::Version};
use rmcp::{ErrorData as McpError, model::CallToolResult, schemars};
use serde::Serialize;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub(crate) struct DependencyTreeArgs {
    /// Name of the crate on crates.io.
    pub(crate) krate: String,
    /// Exact crate version. Use `resolve_version` first if you only have a
    /// semver requirement.
    pub(crate) version: Version,
}

#[derive(Debug, Serialize)]
struct DependencyTreeResult {
    dependencies: Vec<dependency_tree::Dependency>,
}

pub(crate) async fn handle(
    config: &Config,
    args: DependencyTreeArgs,
) -> Result<CallToolResult, McpError> {
    let dependencies =
        dependency_tree::dependency_tree(config, &args.krate, args.version.as_ref())
            .await
            .map_err(|err| McpError::internal_error(err.to_string(), None))?
            .ok_or_else(|| {
                McpError::resource_not_found("crate or version not found on crates.io", None)
            })?;

    Ok(CallToolResult::structured(
        serde_json::to_value(DependencyTreeResult { dependencies })
            .map_err(|err| McpError::internal_error(err.to_string(), None))?,
    ))
}
